#!/usr/bin/env bash
# scripts/release/check.sh
# Verify each prereq tool exists. Exits 1 if any missing, with install hint.
# Argument: space-separated platforms to check (default: all). Recognised:
#   windows linux darwin
set -uo pipefail

REQUESTED="${*:-windows linux darwin}"
MISSING=()

# Required on host ----------------------------------------------------------
need() {
  local cmd="$1" hint="$2"
  if command -v "$cmd" >/dev/null 2>&1; then
    printf '  \033[1;32m✓\033[0m %s\n' "$cmd"
  else
    printf '  \033[1;31m✗\033[0m %s  — %s\n' "$cmd" "$hint"
    MISSING+=("$cmd")
  fi
}

echo "[prereqs] required (every build)"
need bash        "install via package manager"
need cargo       "rustup install stable"
need rustup      "https://rustup.rs/"
need file        "apt install file"
need sha256sum   "coreutils"
need dpkg-deb    "apt install dpkg-dev"
need fakeroot    "apt install fakeroot"

# Icon tools — ImageMagick OR python3+PIL
echo "[prereqs] icon generation"
if command -v magick >/dev/null 2>&1; then
  printf '  \033[1;32m✓\033[0m magick (ImageMagick)\n'
elif command -v convert >/dev/null 2>&1; then
  printf '  \033[1;32m✓\033[0m convert (legacy ImageMagick)\n'
elif python3 -c 'import PIL' 2>/dev/null; then
  printf '  \033[1;32m✓\033[0m python3+PIL (fallback)\n'
else
  printf '  \033[1;31m✗\033[0m icon tools missing — apt install imagemagick python3-pil\n'
  MISSING+=("imagemagick")
fi

# Platform-specific tools ----------------------------------------------------
for plat in $REQUESTED; do
  case "$plat" in
    windows)
      echo "[prereqs] windows"
      need x86_64-w64-mingw32-gcc "apt install gcc-mingw-w64-x86-64"
      if rustup target list --installed | grep -q '^x86_64-pc-windows-gnu$'; then
        printf '  \033[1;32m✓\033[0m %s\n' "x86_64-pc-windows-gnu"
      else
        printf '  \033[1;31m✗\033[0m %s — rustup target add %s\n' \
          "x86_64-pc-windows-gnu" "x86_64-pc-windows-gnu"
        MISSING+=("target:x86_64-pc-windows-gnu")
      fi
      need cargo-wix "cargo install cargo-wix --locked"
      ;;
    linux|linux-x86_64|linux-arm64)
      echo "[prereqs] linux ($plat)"
      if [ "$plat" = "linux-arm64" ]; then
        if rustup target list --installed | grep -q '^aarch64-unknown-linux-gnu$'; then
          printf '  \033[1;32m✓\033[0m %s\n' "aarch64-unknown-linux-gnu"
        else
          printf '  \033[1;31m✗\033[0m %s — rustup target add %s\n' \
            "aarch64-unknown-linux-gnu" "aarch64-unknown-linux-gnu"
          MISSING+=("target:aarch64-unknown-linux-gnu")
        fi
        need aarch64-linux-gnu-gcc "apt install gcc-aarch64-linux-gnu"
      else
        if rustup target list --installed | grep -q '^x86_64-unknown-linux-gnu$'; then
          printf '  \033[1;32m✓\033[0m %s\n' "x86_64-unknown-linux-gnu"
        else
          printf '  \033[1;31m✗\033[0m %s — rustup target add %s\n' \
            "x86_64-unknown-linux-gnu" "x86_64-unknown-linux-gnu"
          MISSING+=("target:x86_64-unknown-linux-gnu")
        fi
        # AppImage only needed for amd64 (advisory — build-linux.sh skips with a WARN)
        if command -v appimagetool >/dev/null 2>&1; then
          printf '  \033[1;32m✓\033[0m %s\n' "appimagetool"
        else
          printf '  \033[1;33m-\033[0m %s missing — AppImage build will be skipped\n' "appimagetool"
        fi
      fi
      ;;
    darwin-x86_64|darwin-arm64|darwin)
      echo "[prereqs] darwin"
      OSXCROSS="${OSXCROSS:-$HOME/osxcross}"
      if [ -d "$OSXCROSS/target" ]; then
        printf '  \033[1;32m✓\033[0m osxcross @ %s\n' "$OSXCROSS"
      else
        printf '  \033[1;31m✗\033[0m osxcross — run %s/build.sh (requires Apple SDK)\n' "$OSXCROSS"
        MISSING+=("osxcross")
      fi
      LIBDMG="${LIBDMG:-$HOME/libdmg-hfsplus}"
      if [ -d "$LIBDMG/build" ]; then
        printf '  \033[1;32m✓\033[0m libdmg-hfsplus @ %s\n' "$LIBDMG"
      else
        printf '  \033[1;33m-\033[0m libdmg-hfsplus missing — .dmg skipped\n'
      fi
      ;;
  esac
done

echo "-----"
if [ ${#MISSING[@]} -ne 0 ]; then
  printf '\033[1;31mMISSING:\033[0m %s\n' "${MISSING[*]}"
  printf 'run: \033[1;33msudo %s/scripts/release/install-prereqs.sh\033[0m\n' "$PWD"
  exit 1
fi
echo "[prereqs] OK"
