#!/usr/bin/env bash
# scripts/release/build-macos.sh
# Cross-compile drz-app for darwin + bundle .app + .dmg.
# Skips silently if osxcross / libdmg-hfsplus not installed.
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

OSXCROSS="${OSXCROSS:-$HOME/osxcross}"
LIBDMG="${LIBDMG:-$HOME/libdmg-hfsplus}"

if [ ! -d "$OSXCROSS/target" ]; then
  echo "[macos] SKIP — osxcross not built. Build $OSXCROSS first." >&2
  exit 0
fi
if [ ! -d "$LIBDMG/build" ]; then
  echo "[macos] SKIP — libdmg-hfsplus not built. Skipping .dmg generation." >&2
  SKIP_DMG=1
fi

# Source osxcross env so clang/wrapper binaries are available
# shellcheck disable=SC1091
. "$OSXCROSS/target/env.sh" 2>/dev/null || true

STAGE="${REPO_ROOT}/releases/${VERSION}/${FOLDER}"
mkdir -p "$STAGE"
echo "[macos] build → $TARGET"
cd "$REPO_ROOT"
cargo build --release --target "$TARGET" -p drz-app --locked

BIN="target/${TARGET}/release/drzdiff"
[ -f "$BIN" ] || { echo "missing $BIN" >&2; exit 1; }

# Build .app bundle ---------------------------------------------------------
APP_DIR="$STAGE/${APP_NAME}.app"
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"
cp "$BIN" "$APP_DIR/Contents/MacOS/${APP_NAME}"
chmod +x "$APP_DIR/Contents/MacOS/${APP_NAME}"

if [ -f "$REPO_ROOT/icons/AppIcon.icns" ]; then
  cp "$REPO_ROOT/icons/AppIcon.icns" "$APP_DIR/Contents/Resources/AppIcon.icns"
fi

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

# Codesign ad-hoc (allows running locally without a Developer ID; not notarized)
codesign --force --deep --sign - "$APP_DIR" 2>/dev/null || echo "WARN: ad-hoc codesign skipped"

# Zip the .app for direct download
( cd "$STAGE" && zip -qr "${APP_NAME}_${VERSION}_${ARCH}.zip" "${APP_NAME}.app" )

# .dmg via libdmg-hfsplus ---------------------------------------------------
if [ -z "${SKIP_DMG:-}" ]; then
  echo "[macos] .dmg"
  DMG_VOL="DRZDiff_${VERSION}_${ARCH}"
  DMG_OUT="${STAGE}/${DMG_VOL}.dmg"

  # Create sparse image
  rm -f /tmp/drz.dmg.sparseimage
  DMG_DIR="$LIBDMG/build/dmg"
  HFS_DIR="$LIBDMG/build/hfs"
  "$DMG_DIR/dmg" create /tmp/drz.dmg.sparseimage "$DMG_VOL" 200
  "$HFS_DIR/hfsplus" /tmp/drz.dmg.sparseimage add "$APP_DIR" \
    "Applications -> /Applications" 2>/dev/null || true
  "$HFS_DIR/hfsplus" /tmp/drz.dmg.sparseimage mkdir Applications
  "$HFS_DIR/hfsplus" /tmp/drz.dmg.sparseimage rmdir Applications 2>/dev/null || true
  "$DMG_DIR/dmg" build /tmp/drz.dmg.sparseimage "$DMG_OUT"
  rm -f /tmp/drz.dmg.sparseimage
fi

# install.sh (a user runs this on macOS) ------------------------------------
cat > "$STAGE/install.sh" <<'SH'
#!/usr/bin/env bash
# install.sh — DRZ Diff for macOS
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

if [ -f "DRZDiff_*_*.dmg" ] 2>/dev/null && ls DRZDiff_*_*.dmg >/dev/null 2>&1; then
  DMG=$(ls DRZDiff_*_*.dmg | head -1)
  echo "Mounting $DMG ..."
  hdiutil attach "$DMG" -nobrowse -quiet
  sleep 1
  MOUNTPOINT=$(ls -d /Volumes/DRZDiff* | head -1)
  cp -R "$MOUNTPOINT/DRZDiff.app" /Applications/
  hdiutil detach "$MOUNTPOINT"
  echo "Installed to /Applications/DRZDiff.app"
elif [ -d "DRZDiff.app" ]; then
  echo "Installing DRZDiff.app to /Applications/ ..."
  cp -R DRZDiff.app /Applications/
  echo "Done."
else
  echo "ERROR: no .dmg or .app found in $(pwd)" >&2
  exit 1
fi
SH
chmod +x "$STAGE/install.sh"

# SHA256SUMS -----------------------------------------------------------------
cd "$STAGE"
sha256sum * > SHA256SUMS
ls -la
echo "[macos] done"
