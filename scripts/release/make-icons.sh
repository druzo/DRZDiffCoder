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
  # Prefer ImageMagick for resize; fall back to PIL if neither magick nor
  # convert is on PATH.
  for sz in 16 32 64 128 256 512; do
    if command -v magick >/dev/null 2>&1; then
      magick "$SRC" -resize "${sz}x${sz}" "$ICONSET/icon_${sz}x${sz}.png"
      magick "$SRC" -resize "$((sz * 2))x$((sz * 2))" "$ICONSET/icon_${sz}x${sz}@2x.png"
    elif command -v convert >/dev/null 2>&1; then
      convert "$SRC" -resize "${sz}x${sz}" "$ICONSET/icon_${sz}x${sz}.png"
      convert "$SRC" -resize "$((sz * 2))x$((sz * 2))" "$ICONSET/icon_${sz}x${sz}@2x.png"
    else
      python3 - "$SRC" "$ICONSET" "$sz" <<'PY'
import sys
from PIL import Image
src, ic, sz = sys.argv[1], sys.argv[2], int(sys.argv[3])
img = Image.open(src).convert("RGBA")
img.resize((sz, sz), Image.LANCZOS).save(f"{ic}/icon_{sz}x{sz}.png", optimize=True)
img.resize((sz*2, sz*2), Image.LANCZOS).save(f"{ic}/icon_{sz}x{sz}@2x.png", optimize=True)
PY
    fi
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
"""
Build a proper Apple .icns file from a single PNG.

ICNS file layout:
  header: 'icns' (4) + total_size (4, big-endian)
  entries: type (4) + size (4, big-endian) + data
PNG entries used (modern macOS):
  icp4  16   ic07  128   ic08  256   ic09  512   ic10 1024
  icp5  32   ic11   32   ic12   64   ic13  256 (128@2x)   ic14 512 (256@2x)
  icp6  64
Sizes we emit: 16, 32, 64, 128, 256, 512 (PNG-encoded).
"""
import sys, struct
from PIL import Image

src, icns, ico = sys.argv[1], sys.argv[2], sys.argv[3]
img = Image.open(src).convert("RGBA")

# Write a multi-size .ico (always, for Windows PE embed)
ico_sizes = [(s, s) for s in (16, 32, 48, 64, 96, 128, 256)]
img.save(ico, format="ICO", sizes=ico_sizes)

# Build .icns
TYPE_SIZE = {
    16:  b"icp4", 32:  b"icp5", 64:  b"icp6",
    128: b"ic07", 256: b"ic08", 512: b"ic09",
    1024: b"ic10",
}
entries = []
for sz in (16, 32, 64, 128, 256, 512, 1024):
    if sz > max(img.size):
        continue
    scaled = img.resize((sz, sz), Image.LANCZOS)
    from io import BytesIO
    buf = BytesIO()
    scaled.save(buf, format="PNG", optimize=True)
    payload = buf.getvalue()
    entries.append((TYPE_SIZE[sz], payload))

# Total = 8 (header) + sum(8 + len(payload))
total = 8 + sum(8 + len(p) for _, p in entries)
with open(icns, "wb") as f:
    f.write(b"icns" + struct.pack(">I", total))
    for typ, payload in entries:
        f.write(typ + struct.pack(">I", 8 + len(payload)) + payload)
print(f"[icons] wrote {icns} with sizes {[s for s,_ in entries]}")
PY
else
  echo "WARN: no iconutil/png2icns/python-pil — skipping .icns" >&2
fi

[ -f "$ICO" ] && echo "  $(file "$ICO" | head -c 100)"
[ -f "$ICNS" ] && echo "  $(file "$ICNS" | head -c 100)"
echo "[icons] done"
