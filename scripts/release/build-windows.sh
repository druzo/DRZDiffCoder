#!/usr/bin/env bash
# scripts/release/build-windows.sh
# Cross-compile drz-app for x86_64 Windows (GNU) + build .msi via cargo-wix.
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
VERSION="${VERSION:?missing VERSION}"
TARGET="${WINDOWS_TARGET:-x86_64-pc-windows-gnu}"
STAGE="${REPO_ROOT}/releases/${VERSION}/windows-x86_64"
BIN_DIR="target/${TARGET}/release"

mkdir -p "$STAGE"
echo "[windows] build → $TARGET"
cd "$REPO_ROOT"
cargo build --release --target "$TARGET" -p drz-app --locked

EXE="$BIN_DIR/drzdiff.exe"
[ -f "$EXE" ] || { echo "missing $EXE" >&2; exit 1; }

# Strip debug info — reduces size by ~70%
strip "$EXE" 2>/dev/null || true

cp "$EXE" "$STAGE/drzdiff.exe"

# WiX installer --------------------------------------------------------------
echo "[windows] cargo wix"
WIX_TMP="$REPO_ROOT/target/wix"
mkdir -p "$WIX_TMP"

# Initialize wix scaffold (idempotent — overwrites if exists)
cargo wix --package-name "drzdiff" --product-name "DRZ Diff" \
  --product-version "$VERSION" --manufacturer "DRZ" \
  --license "$REPO_ROOT/LICENSE" --workdir "$WIX_TMP" --no-build 2>/dev/null || true

# The scaffold lands in crates/drz-app/wix/. Copy and edit.
WIX_SRC="$REPO_ROOT/crates/drz-app/wix"
if [ -d "$WIX_SRC" ]; then
  # Patch the WiX template to install the .exe and create Start Menu shortcut.
  cp "$WIX_SRC/main.wxs" "$WIX_SRC/main.wxs.bak" 2>/dev/null || true
  cat > "$WIX_SRC/main.wxs" <<WXS
<?xml version='1.0'?>
<Wix xmlns='http://schemas.microsoft.com/wix/2006/wi'>
  <Product Id='*' Name='DRZ Diff' Version='${VERSION}' Manufacturer='DRZ'
           UpgradeCode='d4f3b6a1-2c0e-4a8e-9f7a-1b2c3d4e5f60'>
    <Package Description='Source code diff tool' Manufacturer='DRZ'
             InstallerVersion='500' Compressed='yes' />
    <Media Id='1' Cabinet='drzdiff.cab' EmbedCab='yes' />
    <Directory Id='ProgramFilesFolder'>
      <Directory Id='INSTALLDIR' Name='DRZ Diff'>
        <Component Id='MainExecutable' Guid='c5a3b6a1-2c0e-4a8e-9f7a-1b2c3d4e5f60'>
          <File Id='DrzDiffExe' Name='drzdiff.exe' Source='${EXE}' KeyPath='yes' />
        </Component>
      </Directory>
    </Directory>
    <Directory Id='ProgramMenuFolder'>
      <Directory Id='MenuFolder' Name='DRZ Diff'>
        <Component Id='Shortcut' Guid='c5a3b6a2-2c0e-4a8e-9f7a-1b2c3d4e5f60'>
          <Shortcut Id='StartMenuShortcut' Name='DRZ Diff'
                    Target='[INSTALLDIR]drzdiff.exe' />
        </Component>
      </Directory>
    </Directory>
    <Feature Id='MainFeature' Title='DRZ Diff' Level='1'>
      <ComponentRef Id='MainExecutable' />
      <ComponentRef Id='Shortcut' />
    </Feature>
  </Product>
</Wix>
WXS

  cd "$REPO_ROOT/crates/drz-app"
  cargo wix --no-build
  cd "$REPO_ROOT"
  MSI="$(find target/wix -name '*.msi' -type f | head -1)"
  if [ -n "$MSI" ] && [ -f "$MSI" ]; then
    cp "$MSI" "$STAGE/drzdiff_${VERSION}_x64.msi"
    echo "[windows] $(basename "$MSI") copied"
  else
    echo "WARN: WiX did not produce a .msi — manual debug required" >&2
  fi
else
  echo "WARN: cargo wix scaffold not created at $WIX_SRC" >&2
fi

# Install + uninstall scripts ----------------------------------------------
cat > "$STAGE/install.bat" <<'BAT'
@echo off
setlocal
pushd "%~dp0"
if not exist "drzdiff_%VERSION%_x64.msi" (
  echo Missing drzdiff_%VERSION%_x64.msi in %cd%
  popd & exit /b 1
)
echo Installing DRZ Diff %VERSION% ...
msiexec /i "drzdiff_%VERSION%_x64.msi" /qb! ADDLOCAL=ALL
if errorlevel 1 (
  echo Install failed.
  popd & exit /b 1
)
echo.
echo Done. Find "DRZ Diff" in Start Menu.
popd
endlocal
BAT

cat > "$STAGE/uninstall.bat" <<'BAT'
@echo off
setlocal
pushd "%~dp0"
echo Uninstalling DRZ Diff ...
wmic product where "name='DRZ Diff'" call uninstall /nointeractive
if errorlevel 1 (
  echo Remove failed. Try: Settings ^> Apps ^> Installed apps ^> DRZ Diff.
  popd & exit /b 1
)
echo Done.
popd
endlocal
BAT

# Sed-replace %VERSION% literal (Windows batch doesn't expand in heredoc)
sed -i "s/%VERSION%/${VERSION}/g" "$STAGE/install.bat"

# SHA256SUMS ---------------------------------------------------------------
cd "$STAGE"
sha256sum * > SHA256SUMS
ls -la
echo "[windows] done"
