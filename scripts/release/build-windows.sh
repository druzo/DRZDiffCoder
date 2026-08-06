#!/usr/bin/env bash
# scripts/release/build-windows.sh
# Cross-compile drz-app for x86_64 Windows (GNU) + build .msi via cargo-wix.
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
VERSION="${VERSION:?missing VERSION}"
TARGET="${WINDOWS_TARGET:-x86_64-pc-windows-gnu}"
BIN_DIR="target/${TARGET}/release"
CRATE_DIR="$REPO_ROOT/crates/drz-app"

mkdir -p "${REPO_ROOT}/releases/${VERSION}/windows-x86_64"
STAGE="${REPO_ROOT}/releases/${VERSION}/windows-x86_64"

echo "[windows] build → $TARGET"
cd "$REPO_ROOT"
local_cc_var="CC_$(echo "$TARGET" | tr '-' '_')"
export "${local_cc_var}=x86_64-w64-mingw32-gcc"
cargo build --release --target "$TARGET" -p drz-app --locked

EXE="$BIN_DIR/drzdiff.exe"
[ -f "$EXE" ] || { echo "missing $EXE" >&2; exit 1; }
strip "$EXE" 2>/dev/null || true

cp "$EXE" "$STAGE/drzdiff.exe"

# WiX installer -------------------------------------------------------------
echo "[windows] cargo wix init"
cd "$CRATE_DIR"
WIX_DIR="$CRATE_DIR/wix"
rm -rf "$WIX_DIR" target/wix 2>/dev/null || true

cargo wix init --force \
  --product-name "DRZ Diff" \
  --product-version "$VERSION" \
  --manufacturer "DRZ" \
  --license "$REPO_ROOT/LICENSE" \
  --no-build 2>&1 | tail -5 || {
    echo "WARN: cargo wix init failed (check that WiX Toolset is installed)" >&2
  }

if [ -f "$WIX_DIR/main.wxs" ]; then
  # Replace generated WiX with our installer template.
  cat > "$WIX_DIR/main.wxs" <<WXS
<?xml version='1.0'?>
<Wix xmlns='http://schemas.microsoft.com/wix/2006/wi'>
  <Product Id='*' Name='DRZ Diff' Version='${VERSION}' Manufacturer='DRZ'
           Language='1033' UpgradeCode='d4f3b6a1-2c0e-4a8e-9f7a-1b2c3d4e5f60'>
    <Package Description='Source code diff tool' Manufacturer='DRZ'
             InstallerVersion='500' Compressed='yes' />
    <Media Id='1' Cabinet='drzdiff.cab' EmbedCab='yes' />
    <Directory Id='TARGETDIR' Name='SourceDir'>
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
    </Directory>
    <Feature Id='MainFeature' Title='DRZ Diff' Level='1'>
      <ComponentRef Id='MainExecutable' />
      <ComponentRef Id='Shortcut' />
    </Feature>
  </Product>
</Wix>
WXS

  echo "[windows] cargo wix build"
  if cargo wix --no-build 2>&1 | tail -5; then
    MSI="$(find target/wix -name '*.msi' -type f 2>/dev/null | head -1)"
    if [ -n "$MSI" ] && [ -f "$MSI" ]; then
      cp "$MSI" "$STAGE/drzdiff_${VERSION}_x64.msi"
      echo "[windows] $(basename "$MSI") copied"
    else
      echo "WARN: cargo wix succeeded but no .msi found in target/wix/" >&2
    fi
  else
    echo "WARN: cargo wix build failed — likely missing WiX Toolset (candle.exe + light.exe)" >&2
    echo "  install-prereqs.sh installs wixl as a Linux-native fallback" >&2
    echo "  building .msi requires a Windows host or Wine + WiX 3.14 — see spec" >&2
  fi
else
  echo "WARN: wix/main.wxs not created — .msi skipped" >&2
fi

cd "$REPO_ROOT"

# Install + uninstall scripts ----------------------------------------------
# install.bat: prefers .msi installer; falls back to direct file copy if .msi
# is absent (e.g. when built on Linux+Wine where light.exe cannot link).
cat > "$STAGE/install.bat" <<BAT
@echo off
REM install.bat - DRZ Diff ${VERSION}
setlocal
pushd "%~dp0"

if exist "drzdiff_${VERSION}_x64.msi" goto :msi_install

echo --- .msi not found, installing via direct copy ---
set "INSTALLDIR=%LOCALAPPDATA%\Programs\DRZ Diff"
if not exist "%INSTALLDIR%" mkdir "%INSTALLDIR%"
copy /Y "drzdiff.exe" "%INSTALLDIR%\drzdiff.exe" >NUL
echo.
echo Installed DRZ Diff ${VERSION} to %INSTALLDIR%.
echo To create a Start Menu shortcut, run install_shortcut.ps1 (PowerShell).
popd
endlocal
exit /b 0

:msi_install
echo Installing DRZ Diff ${VERSION} via .msi...
msiexec /i "drzdiff_${VERSION}_x64.msi" /qb! ADDLOCAL=ALL
if errorlevel 1 ( echo Install failed.& popd & exit /b 1 )
echo Done. Find "DRZ Diff" in Start Menu.
popd
endlocal
exit /b 0
BAT

# PowerShell companion for the no-.msi path: creates a Start Menu shortcut.
cat > "$STAGE/install_shortcut.ps1" <<PS1
\$ErrorActionPreference = 'Stop'
\$InstallDir = Join-Path \$env:LOCALAPPDATA 'Programs\DRZ Diff'
\$Exe = Join-Path \$InstallDir 'drzdiff.exe'
if (-not (Test-Path \$Exe)) { Write-Error "drzdiff.exe not found at \$Exe — run install.bat first"; exit 1 }
\$ShortcutDir = Join-Path \$env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
if (-not (Test-Path \$ShortcutDir)) { New-Item -ItemType Directory -Path \$ShortcutDir | Out-Null }
\$ShortcutPath = Join-Path \$ShortcutDir 'DRZ Diff.lnk'
\$Shell = New-Object -ComObject WScript.Shell
\$Shortcut = \$Shell.CreateShortcut(\$ShortcutPath)
\$Shortcut.TargetPath = \$Exe
\$Shortcut.WorkingDirectory = \$InstallDir
\$Shortcut.IconLocation = "\$(\$Exe),0"
\$Shortcut.Save()
Write-Host "Created Start Menu shortcut: \$ShortcutPath"
PS1

cat > "$STAGE/uninstall.bat" <<'BAT'
@echo off
REM uninstall.bat - DRZ Diff
setlocal
pushd "%~dp0"

if exist "drzdiff_*_x64.msi" (
  for %%M in (drzdiff_*_x64.msi) do (
    echo Uninstalling via .msi: %%M
    msiexec /x "%%M" /qb!
  )
) else (
  set "INSTALLDIR=%LOCALAPPDATA%\Programs\DRZ Diff"
  if exist "%INSTALLDIR%\drzdiff.exe" del "%INSTALLDIR%\drzdiff.exe"
  if exist "%INSTALLDIR%" rmdir "%INSTALLDIR%" 2>NUL
  if exist "%APPDATA%\Microsoft\Windows\Start Menu\Programs\DRZ Diff.lnk" del "%APPDATA%\Microsoft\Windows\Start Menu\Programs\DRZ Diff.lnk"
  echo Removed DRZ Diff from %INSTALLDIR%.
)
popd
endlocal
exit /b 0
BAT

cd "$STAGE"
sha256sum * > SHA256SUMS
ls -la
echo "[windows] done"
