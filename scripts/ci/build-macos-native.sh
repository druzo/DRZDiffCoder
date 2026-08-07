#!/usr/bin/env bash
# scripts/ci/build-macos-native.sh
# Native macOS build for CI. Produces:
#   - DRZDiff.app bundle
#   - DRZDiff_<VERSION>_<ARCH>.dmg
#   - DRZDiff_<VERSION>_<ARCH>.zip
#   - install.sh
#   - SHA256SUMS
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
VERSION="${VERSION:?missing VERSION}"
ARCH="${1:?usage: $0 <x86_64|arm64>}"
APP_NAME="DRZDiff"
BUNDLE_ID="app.drzdiff"

case "$ARCH" in
  x86_64) TARGET="x86_64-apple-darwin"; FOLDER="darwin-x86_64" ;;
  arm64)  TARGET="aarch64-apple-darwin"; FOLDER="darwin-arm64" ;;
  *) echo "unknown arch $ARCH" >&2; exit 1 ;;
esac

# Override the osxcross linker from .cargo/config.toml on native macOS runners.
LINKER_ENV="CARGO_TARGET_$(echo "$TARGET" | tr 'a-z-' 'A-Z_')_LINKER"
export "$LINKER_ENV"=clang

cd "$REPO_ROOT"
mkdir -p "releases/${VERSION}/${FOLDER}"
STAGE="${REPO_ROOT}/releases/${VERSION}/${FOLDER}"

echo "[macos] building $TARGET -> $FOLDER (linker=$LINKER_ENV=${!LINKER_ENV})"

rustup target add "$TARGET"

cargo build --release --target "$TARGET" -p drz-app --locked

BIN="target/${TARGET}/release/drzdiff"
[ -f "$BIN" ] || { echo "missing $BIN" >&2; exit 1; }

# Generate .icns if missing (requires ImageMagick; usually committed already).
ICNS="$REPO_ROOT/icons/AppIcon.icns"
if [ ! -f "$ICNS" ]; then
  echo "[macos] generating AppIcon.icns"
  if command -v magick >/dev/null 2>&1 || command -v convert >/dev/null 2>&1; then
    "$REPO_ROOT/scripts/release/make-icons.sh"
  else
    echo "ERROR: AppIcon.icns is missing and ImageMagick is not installed" >&2
    exit 1
  fi
fi

# Build .app bundle ---------------------------------------------------------
APP_DIR="$STAGE/${APP_NAME}.app"
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"
cp "$BIN" "$APP_DIR/Contents/MacOS/${APP_NAME}"
chmod +x "$APP_DIR/Contents/MacOS/${APP_NAME}"

[ -f "$ICNS" ] && cp "$ICNS" "$APP_DIR/Contents/Resources/AppIcon.icns"

cat > "$APP_DIR/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>          <string>en</string>
  <key>CFBundleExecutable</key>                <string>${APP_NAME}</string>
  <key>CFBundleIconFile</key>                  <string>AppIcon.icns</string>
  <key>CFBundleIconName</key>                  <string>AppIcon</string>
  <key>CFBundleIdentifier</key>                <string>${BUNDLE_ID}</string>
  <key>CFBundleInfoDictionaryVersion</key>     <string>6.0</string>
  <key>CFBundleName</key>                      <string>DRZ Diff</string>
  <key>CFBundleDisplayName</key>               <string>DRZ Diff</string>
  <key>CFBundlePackageType</key>               <string>APPL</string>
  <key>CFBundleShortVersionString</key>        <string>${VERSION}</string>
  <key>CFBundleVersion</key>                   <string>${VERSION}</string>
  <key>LSMinimumSystemVersion</key>            <string>11.0</string>
  <key>LSApplicationCategoryType</key>         <string>public.app-category.developer-tools</string>
  <key>NSHighResolutionCapable</key>           <true/>
</dict>
</plist>
EOF

# Ad-hoc sign
codesign --force --deep --sign - "$APP_DIR" 2>/dev/null || echo "[macos] ad-hoc codesign skipped"

# Zip the .app
( cd "$STAGE" && zip -qr "${APP_NAME}_${VERSION}_${ARCH}.zip" "${APP_NAME}.app" )

# Build .dmg with hdiutil ---------------------------------------------------
DMG_VOL="DRZDiff ${VERSION}"
DMG_OUT="${STAGE}/DRZDiff_${VERSION}_${ARCH}.dmg"
STAGE_TMP="$STAGE/.dmg-stage"
rm -rf "$STAGE_TMP"
mkdir -p "$STAGE_TMP/Applications"
cp -R "$APP_DIR" "$STAGE_TMP/Applications/"

hdiutil create -volname "$DMG_VOL" -srcfolder "$STAGE_TMP" -ov -format UDZO "$DMG_OUT"
rm -rf "$STAGE_TMP"

# The .app bundle is already inside the .zip and .dmg; remove the raw directory
# so the artifact upload only contains release-ready files.
rm -rf "$APP_DIR"

# install.sh ----------------------------------------------------------------
cat > "$STAGE/install.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

install_app() {
  local app="DRZDiff.app"
  echo "Installing $app to /Applications ..."
  if [ -d "/Applications/$app" ]; then
    rm -rf "/Applications/$app"
  fi
  cp -R "$app" /Applications/
  xattr -dr com.apple.quarantine "/Applications/$app" 2>/dev/null || true
  echo "Done. Open: open /Applications/$app"
}

if ls DRZDiff_*_*.dmg >/dev/null 2>&1; then
  DMG=$(ls DRZDiff_*_*.dmg | head -1)
  echo "Mounting $DMG ..."
  MOUNT=$(hdiutil attach "$DMG" -nobrowse -quiet | tail -1 | awk '{print $NF}')
  sleep 1
  cp -R "$MOUNT/DRZDiff.app" /Applications/
  hdiutil detach "$MOUNT" -quiet || true
  xattr -dr com.apple.quarantine "/Applications/DRZDiff.app" 2>/dev/null || true
  echo "Installed to /Applications/DRZDiff.app"
elif [ -d "DRZDiff.app" ]; then
  install_app
else
  echo "ERROR: no .dmg or .app found in $(pwd)" >&2
  exit 1
fi
SH
chmod +x "$STAGE/install.sh"

# SHA256SUMS -----------------------------------------------------------------
cd "$STAGE"
{
  for f in *; do
    [ "$f" = "SHA256SUMS" ] && continue
    [ -f "$f" ] && sha256sum "$f"
  done
} | sort -u > SHA256SUMS

ls -la

echo "[macos] $FOLDER done"
