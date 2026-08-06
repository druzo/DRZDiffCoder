#!/usr/bin/env bash
# scripts/release/build-linux.sh
# Cross-compile drz-app for x86_64 + aarch64 Linux; produce .deb (both)
# and .AppImage (amd64 only).
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
VERSION="${VERSION:?missing VERSION}"

build_target() {
  local target="$1" folder="$2" deb_arch="$3" appimage="$4"
  echo "[linux] build → $target ($folder)"
  cd "$REPO_ROOT"
  # Set the cross C compiler for cc-rs / build scripts.
  local cc_var="CC_$(echo "$target" | tr '-' '_')"
  local cxx_var="CXX_$(echo "$target" | tr '-' '_')"
  case "$target" in
    x86_64-unknown-linux-gnu)
      export "${cc_var}=cc"
      export "${cxx_var}=c++"
      ;;
    aarch64-unknown-linux-gnu)
      export "${cc_var}=aarch64-linux-gnu-gcc"
      export "${cxx_var}=aarch64-linux-gnu-g++"
      ;;
  esac
  cargo build --release --target "$target" -p drz-app --locked
  local bin="target/${target}/release/drzdiff"
  [ -f "$bin" ] || { echo "missing $bin" >&2; return 1; }
  # Use target-aware strip so cross-built binaries get stripped.
  case "$target" in
    x86_64-unknown-linux-gnu)  strip "$bin" 2>/dev/null || true ;;
    aarch64-unknown-linux-gnu) aarch64-linux-gnu-strip "$bin" 2>/dev/null || true ;;
  esac

  local stage="${REPO_ROOT}/releases/${VERSION}/${folder}"
  rm -rf "$stage"
  mkdir -p "$stage"
  cp "$bin" "$stage/drzdiff"
  chmod +x "$stage/drzdiff"

  # ---- .deb ----------------------------------------------------------------
  echo "[linux] .deb ($deb_arch)"
  build_deb "$stage" "$target" "$deb_arch"

  # ---- AppImage (amd64 only) ----------------------------------------------
  if [ "$appimage" = "yes" ]; then
    echo "[linux] AppImage"
    build_appimage "$stage"
  fi

  # ---- install.sh --------------------------------------------------------
  cat > "$stage/install.sh" <<'SH'
#!/usr/bin/env bash
# install.sh — DRZ Diff for Linux
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"
ARCH=$(uname -m)

case "$ARCH" in
  x86_64|amd64) DEB=$(ls drzdiff_*_amd64.deb 2>/dev/null | head -1);;
  aarch64|arm64) DEB=$(ls drzdiff_*_arm64.deb 2>/dev/null | head -1);;
  *) echo "unknown arch $ARCH" >&2; exit 1 ;;
esac

if [ -n "$DEB" ] && [ -f "$DEB" ]; then
  echo "Installing $DEB via dpkg ..."
  sudo dpkg -i "$DEB"
  if command -v apt-get >/dev/null 2>&1; then
    sudo apt-get install -f -y || true
  fi
  echo "Done. Run: drzdiff"
elif [ -f "drzdiff" ]; then
  APPIMAGE=$(ls drzdiff-*.AppImage 2>/dev/null | head -1)
  if [ -n "$APPIMAGE" ]; then
    echo "Detected AppImage in $SCRIPT_DIR"
    chmod +x "$APPIMAGE"
    ln -sf "$SCRIPT_DIR/$APPIMAGE" "$SCRIPT_DIR/drzdiff"
    if [ -w /usr/local/bin ]; then
      sudo cp "$APPIMAGE" /usr/local/bin/drzdiff
      echo "Installed to /usr/local/bin/drzdiff"
    fi
    echo "Run: ./drzdiff (or ./$APPIMAGE)"
  else
    echo "Local binary at $(pwd)/drzdiff — add to PATH or symlink to /usr/local/bin/"
  fi
else
  echo "No .deb or AppImage found in $SCRIPT_DIR" >&2
  exit 1
fi
SH
  chmod +x "$stage/install.sh"

  cd "$stage"
  sha256sum * > SHA256SUMS
  ls -la
  echo "[linux] $folder done"
}

build_deb() {
  local stage="$1" target="$2" arch="$3"
  local pkgroot="$stage/deb-build"
  rm -rf "$pkgroot"
  mkdir -p "$pkgroot/DEBIAN"
  mkdir -p "$pkgroot/usr/bin"
  mkdir -p "$pkgroot/usr/share/icons/hicolor/256x256/apps"
  mkdir -p "$pkgroot/usr/share/icons/hicolor/128x128/apps"
  mkdir -p "$pkgroot/usr/share/icons/hicolor/48x48/apps"
  mkdir -p "$pkgroot/usr/share/applications"
  mkdir -p "$pkgroot/usr/share/doc/drzdiff"

  install -m755 "$stage/drzdiff" "$pkgroot/usr/bin/drzdiff"
  cp "$REPO_ROOT/icons/AppIcon.png" "$pkgroot/usr/share/icons/hicolor/256x256/apps/drzdiff.png"
  cp "$REPO_ROOT/icons/AppIcon.png" "$pkgroot/usr/share/icons/hicolor/128x128/apps/drzdiff.png"
  cp "$REPO_ROOT/icons/AppIcon.png" "$pkgroot/usr/share/icons/hicolor/48x48/apps/drzdiff.png"
  cp "$REPO_ROOT/LICENSE" "$pkgroot/usr/share/doc/drzdiff/copyright"
  gzip -c -9 "$REPO_ROOT/LICENSE" > "$pkgroot/usr/share/doc/drzdiff/copyright.gz" 2>/dev/null || true

  cat > "$pkgroot/usr/share/applications/drzdiff.desktop" <<EOF
[Desktop Entry]
Name=DRZ Diff
Comment=Source code diff comparer
Exec=drzdiff %U
Icon=drzdiff
Type=Application
Terminal=false
Categories=Development;Utility;
StartupWMClass=drzdiff
MimeType=text/plain;text/x-rust;text/x-python;text/x-c;text/x-c++text/javascript;
EOF

  cat > "$pkgroot/DEBIAN/control" <<EOF
Package: drzdiff
Version: ${VERSION}
Section: devel
Priority: optional
Architecture: ${arch}
Depends: libgtk-3-0, libxcb-render0, libxcb-shape0, libxcb-xfixes0, libdbus-1-3, libatk1.0-0, libatk-bridge2.0-0, libxkbcommon0, libatspi2.0-0
Maintainer: DRZ <noreply@drzdiff.local>
Description: Source code diff/compare tool with tree-sitter highlighting
 DRZ Diff provides side-by-side source comparison with inline editing,
 language-aware syntax highlighting, and merge arrows. Built with Rust +
 egui + tree-sitter.
EOF

  cat > "$pkgroot/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database /usr/share/applications || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q /usr/share/icons/hicolor || true
fi
exit 0
EOF
  chmod 755 "$pkgroot/DEBIAN/postinst"

  cat > "$pkgroot/DEBIAN/prerm" <<'EOF'
#!/bin/sh
set -e
exit 0
EOF
  chmod 755 "$pkgroot/DEBIAN/prerm"

  # Build .deb with fakeroot (no root required for dpkg-deb --build --root-owner-group)
  mkdir -p "$pkgroot/DEBIAN"
  fakeroot dpkg-deb --build --root-owner-group "$pkgroot" \
    "$stage/drzdiff_${VERSION}_${arch}.deb"
  rm -rf "$pkgroot"
}

build_appimage() {
  local stage="$1"
  local appdir="$stage/AppDir"
  rm -rf "$appdir"
  mkdir -p "$appdir/usr/bin" "$appdir/usr/share/icons/hicolor/256x256/apps" \
           "$appdir/usr/share/applications"

  install -m755 "$stage/drzdiff" "$appdir/usr/bin/drzdiff"
  cp "$REPO_ROOT/icons/AppIcon.png" "$appdir/usr/share/icons/hicolor/256x256/apps/drzdiff.png"

  cat > "$appdir/drzdiff.desktop" <<EOF
[Desktop Entry]
Name=DRZ Diff
Comment=Source code diff comparer
Exec=drzdiff %U
Icon=drzdiff
Type=Application
Terminal=false
Categories=Development;Utility;
EOF

  cp "$appdir/drzdiff.desktop" "$appdir/usr/share/applications/drzdiff.desktop"

  cat > "$appdir/AppRun" <<'EOF'
#!/bin/sh
exec "$(dirname "$0")/usr/bin/drzdiff" "$@"
EOF
  chmod +x "$appdir/AppRun"

  if [ -f "$REPO_ROOT/icons/AppIcon.png" ]; then
    cp "$REPO_ROOT/icons/AppIcon.png" "$appdir/drzdiff.png"
    cp "$REPO_ROOT/icons/AppIcon.png" "$appdir/.DirIcon" 2>/dev/null || true
  fi

  local appimagetool="$HOME/.local/bin/appimagetool"
  if [ -x "$appimagetool" ]; then
    ARCH=x86_64 "$appimagetool" "$appdir" "$stage/drzdiff-${VERSION}-x86_64.AppImage" 2>&1 | tail -3
    rm -rf "$appdir"
  else
    echo "WARN: appimagetool not found — AppImage skipped" >&2
    rm -rf "$appdir"
  fi
}

# Build for requested arches only
LINUX_ONLY="${LINUX_ONLY:-}"
case "$LINUX_ONLY" in
  x86_64)
    build_target "x86_64-unknown-linux-gnu" "linux-x86_64" "amd64" "yes"
    ;;
  arm64)
    build_target "aarch64-unknown-linux-gnu" "linux-arm64" "arm64" "no"
    ;;
  *)
    build_target "x86_64-unknown-linux-gnu" "linux-x86_64" "amd64" "yes"
    build_target "aarch64-unknown-linux-gnu" "linux-arm64" "arm64" "no"
    ;;
esac

echo "[linux] all done"
