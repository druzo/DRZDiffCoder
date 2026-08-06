#!/usr/bin/env bash
# scripts/release/make-icons.sh
# Idempotent: regenerates .ico + .icns from icons/AppIcon.png.
# Uses ImageMagick when available; falls back to Python+PIL.
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
SRC="$REPO_ROOT/icons/AppIcon.png"
ICO="$REPO_ROOT/icons/AppIcon.ico"
ICNS="$REPO_ROOT/icons/AppIcon.icns"
ICONSET="$REPO_ROOT/icons/AppIcon.iconset"

[ -f "$SRC" ] || { echo "missing $SRC" >&2; exit 1; }

mkdir -p "$REPO_ROOT/icons"

# --- .ico (multi-size) ----------------------------------------------------
echo "[icons] $ICO"
if command -v magick >/dev/null 2>&1; then
  magick "$SRC" -define icon:auto-resize=256,128,96,64,48,32,16 "$ICO"
elif command -v convert >/dev/null 2>&1; then
  convert "$SRC" -define icon:auto-resize=256,128,96,64,48,32,16 "$ICO"
else
  python3 - <<'PY' "$SRC" "$ICO"
import sys
from PIL import Image
src, dst = sys.argv[1], sys.argv[2]
img = Image.open(src).convert("RGBA")
sizes = [(s, s) for s in (16, 32, 48, 64, 96, 128, 256)]
img.save(dst, format="ICO", sizes=sizes)
PY
fi

# --- .icns ------------------------------------------------------------------
echo "[icons] $ICNS"
if command -v iconutil >/dev/null 2>&1; then
  rm -rf "$ICONSET"
  mkdir -p "$ICONSET"
  for sz in 16 32 64 128 256 512; do
    magick "$SRC" -resize "${sz}x${sz}" "$ICONSET/icon_${sz}x${sz}.png"
    magick "$SRC" -resize "$((sz * 2))x$((sz * 2))" "$ICONSET/icon_${sz}x${sz}@2x.png"
  done
  iconutil -c icns "$ICONSET" -o "$ICNS"
  rm -rf "$ICONSET"
elif command -v png2icns >/dev/null 2>&1; then
  png2icns "$ICNS" \
    "$SRC" 16 16 \
    "$SRC" 32 32 \
    "$SRC" 64 64 \
    "$SRC" 128 128 \
    "$SRC" 256 256 \
    "$SRC" 512 512
elif python3 -c "import PIL" 2>/dev/null; then
  python3 - <<'PY' "$SRC" "$ICNS" "$ICO"
import sys, os
from PIL import Image
src, icns, ico = sys.argv[1], sys.argv[2], sys.argv[3]
img = Image.open(src).convert("RGBA")
sizes = [(s, s) for s in (16, 32, 48, 64, 96, 128, 256)]
img.save(ico, format="ICO", sizes=sizes)
# Python has no native .icns encoder. Write the same .ico bytes to .icns —
# build_macos.sh tolerates this and CFBundleIconFile will point at it; on
# macOS the dock/launchpad falls back to the embedded PNG via with_icon.
with open(icns, "wb") as f:
    f.write(open(ico, "rb").read())
print("[icons] NOTE: macOS-native .icns unavailable — copied .ico as fallback")
PY
else
  echo "WARN: no iconutil/png2icns/python-pil — skipping .icns" >&2
fi

[ -f "$ICO" ] && echo "  $(file "$ICO" | head -c 100)"
[ -f "$ICNS" ] && echo "  $(file "$ICNS" | head -c 100)"
echo "[icons] done"
