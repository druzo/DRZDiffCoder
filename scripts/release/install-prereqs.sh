#!/usr/bin/env bash
# scripts/release/install-prereqs.sh
# One-time setup. Installs all toolchains needed by scripts/release.sh.
# Idempotent: re-running skips already-installed pieces.
set -euo pipefail

SUDO=""
if [ "$(id -u)" -ne 0 ]; then
  if command -v sudo >/dev/null 2>&1; then
    SUDO="sudo"
  else
    echo "ERROR: not root and no sudo. Re-run as root or install sudo." >&2
    exit 1
  fi
fi

log() { printf '\033[1;36m[prereqs]\033[0m %s\n' "$*"; }

# 1. apt packages ---------------------------------------------------------
APT_PKGS=(
  binutils-mingw-w64-x86-64
  gcc-mingw-w64-x86-64
  gcc-aarch64-linux-gnu
  g++-aarch64-linux-gnu
  imagemagick
  dpkg-dev
  fakeroot
  libfuse2t64
  libarchive-tools
  cmake
  libssl-dev
  pkg-config
  wget
  curl
  git
  file
  python3
  python3-pil
  wixl
  rpm
)
log "apt install (${#APT_PKGS[@]} pkgs)"
if [ "$(id -u)" -ne 0 ] && ! $SUDO -n true 2>/dev/null; then
  log "  SKIP — sudo requires interactive auth. Re-run with: sudo $0"
else
  $SUDO apt-get update -qq
  $SUDO apt-get install -y --no-install-recommends "${APT_PKGS[@]}"
fi

# 2. rustup targets -------------------------------------------------------
log "rustup targets"
RUSTUP_TARGETS=(
  x86_64-unknown-linux-gnu
  aarch64-unknown-linux-gnu
  x86_64-pc-windows-gnu
  x86_64-apple-darwin
  aarch64-apple-darwin
)
for t in "${RUSTUP_TARGETS[@]}"; do
  if rustup target list --installed | grep -q "^${t}$"; then
    log "  ✓ $t"
  else
    log "  + $t"
    rustup target add "$t"
  fi
done

# 3. cargo tools ----------------------------------------------------------
log "cargo install cargo-wix"
if ! command -v cargo-wix >/dev/null 2>&1; then
  cargo install cargo-wix --locked
fi
log "  (winresource is a library — declared in Cargo.toml, not installed here)"

# 4. third-party tarballs -------------------------------------------------
export PATH="$HOME/.local/bin:$PATH"
mkdir -p "$HOME/.local/bin" "$HOME/.cache/drzdiff-tools"

# appimagetool (legacy AppImageKit — supports `appimagetool SRC DEST` CLI)
APPIMAGE_TOOL="$HOME/.local/bin/appimagetool"
if [ ! -x "$APPIMAGE_TOOL" ]; then
  log "download appimagetool (legacy)"
  curl -fL -o "$APPIMAGE_TOOL" \
    "https://github.com/AppImage/AppImageKit/releases/download/13/obsolete-appimagetool-x86_64.AppImage"
  chmod +x "$APPIMAGE_TOOL"
fi

# png2icns (for .icns generation when iconutil unavailable)
PNG2ICNS="$HOME/.local/bin/png2icns"
if [ ! -x "$PNG2ICNS" ]; then
  log "download png2icns"
  curl -fL https://github.com/bitboss101/png2icns/releases/download/v1.0/png2icns-x86_64 \
    -o "$PNG2ICNS" 2>/dev/null || log "  png2icns unavailable — falling back to ImageMagick"
  [ -x "$PNG2ICNS" ] && chmod +x "$PNG2ICNS"
fi

# 5. osxcross (optional — enables macOS targets) -------------------------
OSXCROSS="$HOME/osxcross"
if [ ! -d "$OSXCROSS" ]; then
  log "clone osxcross (optional; macOS targets only)"
  git clone --depth 1 https://github.com/tpoechtrager/osxcross.git "$OSXCROSS"
  log "  fetch SDK via osxcross/build_sdk.sh — requires accepting Apple's EULA"
  log "  leave the SDK directory empty if you only build Linux/Windows today"
fi

# 6. libdmg-hfsplus (optional — enables .dmg creation) -------------------
LIBDMG="$HOME/libdmg-hfsplus"
if [ ! -d "$LIBDMG" ]; then
  log "clone libdmg-hfsplus (optional; .dmg creation only)"
  git clone --depth 1 https://github.com/fanquake/libdmg-hfsplus.git "$LIBDMG"
  ( cd "$LIBDMG" && cmake -B build . && cmake --build build -j"$(nproc)" )
  log "  libdmg-hfsplus built at $LIBDMG/build"
fi

log "ALL DONE"
log "add to ~/.bashrc:  export PATH=\"\$HOME/.local/bin:\$PATH\""
log "and (if using osxcross):  source $OSXCROSS/target/env.sh"
