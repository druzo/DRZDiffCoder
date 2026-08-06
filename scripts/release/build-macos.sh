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
export PATH="$OSXCROSS/target/bin:$PATH"

# Map CC_*/CXX_* env vars to the osxcross clang wrappers. cc-rs (used by
# tree-sitter, ring, etc.) reads CC_<target-with-dashes-underscored>.
case "$ARCH" in
  x86_64) OSXCROSS_CC="x86_64-apple-darwin20.4-clang" ;;
  arm64)  OSXCROSS_CC="aarch64-apple-darwin20.4-clang" ;;
esac
export "CC_$(echo "$TARGET" | tr '-' '_')=$OSXCROSS_CC"
export "CXX_$(echo "$TARGET" | tr '-' '_')=${OSXCROSS_CC}++"
export MACOSX_DEPLOYMENT_TARGET=11.0

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

# .dmg via genisoimage + libdmg-hfsplus -------------------------------------
# genisoimage (from cdrkit, apt-installed) creates a hybrid ISO9660/HFS+
# filesystem containing DRZDiff.app. libdmg-hfsplus's `dmg` then wraps that
# ISO in the Apple DMG container (koly trailer).
if [ -z "${SKIP_DMG:-}" ]; then
  echo "[macos] .dmg"
  GENISOIMAGE="$(command -v genisoimage || echo "$HOME/.local/cdrkit/usr/bin/genisoimage")"
  if [ ! -x "$GENISOIMAGE" ]; then
    echo "WARN: genisoimage not found — .dmg skipped (install cdrkit)" >&2
  else
    DMG_VOL="DRZDiff ${VERSION}"
    DMG_OUT="${STAGE}/DRZDiff_${VERSION}_${ARCH}.dmg"
    STAGE_TMP="$STAGE/.dmg-stage"
    rm -rf "$STAGE_TMP" /tmp/drz.iso
    mkdir -p "$STAGE_TMP/Applications"
    cp -R "$APP_DIR" "$STAGE_TMP/Applications/"
    ln -snf Applications "$STAGE_TMP/Applications_DRZDiff" 2>/dev/null || true

    "$GENISOIMAGE" -V "$DMG_VOL" -no-pad -r -apple \
      -o /tmp/drz.iso "$STAGE_TMP" 2>&1 | tail -3
    "$LIBDMG/build/dmg/dmg" /tmp/drz.iso "$DMG_OUT" 2>&1 | tail -3
    rm -f /tmp/drz.iso
    rm -rf "$STAGE_TMP"
    ls -la "$DMG_OUT" 2>&1 | head -1
  fi
fi

# install.sh (a user runs this on macOS) ------------------------------------
cat > "$STAGE/install.sh" <<'SH'
#!/usr/bin/env bash
# install.sh — DRZ Diff for macOS
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
  hdiutil attach "$DMG" -nobrowse -quiet
  sleep 1
  MOUNTPOINT=$(ls -d /Volumes/DRZDiff* /Volumes/"DRZDiff "* 2>/dev/null | head -1)
  if [ -z "$MOUNTPOINT" ]; then
    echo "ERROR: mountpoint not found" >&2
    exit 1
  fi
  cp -R "$MOUNTPOINT/DRZDiff.app" /Applications/
  hdiutil detach "$MOUNTPOINT" || true
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
# Include both top-level files and the DRZDiff.app bundle contents so users
# can verify the .app byte-for-byte against the checksum.
{
  # Use a non-strict `*` so `sha256sum` doesn't exit non-zero on directories.
  for f in *; do
    [ "$f" = "SHA256SUMS" ] && continue
    [ -f "$f" ] && sha256sum "$f"
  done
  find DRZDiff.app -type f -exec sha256sum {} +
} | sort -u > SHA256SUMS
cat SHA256SUMS
echo "[macos] done"
