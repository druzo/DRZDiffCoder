# scripts/ci/build-windows-native.ps1
# Native Windows build for CI on windows-latest. Produces:
#   - drzdiff.exe
#   - drzdiff_<VERSION>_x64.msi
#   - install.bat / install_shortcut.ps1 / uninstall.bat
#   - SHA256SUMS
param(
    [Parameter(Mandatory)]
    [string]$Version
)

$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$Stage = "$RepoRoot/releases/$Version/windows-x86_64"
New-Item -ItemType Directory -Force -Path $Stage | Out-Null

Push-Location $RepoRoot

try {
    # Ensure target
    rustup target add x86_64-pc-windows-msvc

    # Build release binary
    cargo build --release --target x86_64-pc-windows-msvc -p drz-app --locked

    $Exe = "$RepoRoot/target/x86_64-pc-windows-msvc/release/drzdiff.exe"
    if (-not (Test-Path $Exe)) { throw "missing $Exe" }
    Copy-Item $Exe "$Stage/drzdiff.exe" -Force

    # Install cargo-wix and build MSI
    cargo install cargo-wix --locked

    $CrateDir = "$RepoRoot/crates/drz-app"
    $WixDir = "$CrateDir/wix"
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $WixDir
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue "$CrateDir/target/wix"

    Push-Location $CrateDir
    cargo wix init --force `
        --product-name "DRZ Diff" `
        --product-version $Version `
        --manufacturer "DRZ" `
        --license "$RepoRoot/LICENSE" `
        --no-build 2>&1 | Select-Object -Last 5

    if (Test-Path "$WixDir/main.wxs") {
        @"
<?xml version='1.0'?>
<Wix xmlns='http://schemas.microsoft.com/wix/2006/wi'>
  <Product Id='*' Name='DRZ Diff' Version='$Version' Manufacturer='DRZ'
           Language='1033' UpgradeCode='d4f3b6a1-2c0e-4a8e-9f7a-1b2c3d4e5f60'>
    <Package Description='Source code diff tool' Manufacturer='DRZ'
             InstallerVersion='500' Compressed='yes' />
    <Media Id='1' Cabinet='drzdiff.cab' EmbedCab='yes' />
    <Directory Id='TARGETDIR' Name='SourceDir'>
      <Directory Id='ProgramFilesFolder'>
        <Directory Id='INSTALLDIR' Name='DRZ Diff'>
          <Component Id='MainExecutable' Guid='c5a3b6a1-2c0e-4a8e-9f7a-1b2c3d4e5f60'>
            <File Id='DrzDiffExe' Name='drzdiff.exe' Source='$Exe' KeyPath='yes' />
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
"@ | Set-Content "$WixDir/main.wxs" -Encoding UTF8

        cargo wix --no-build 2>&1 | Select-Object -Last 5
        $Msi = Get-ChildItem "$CrateDir/target/wix" -Filter '*.msi' -File | Select-Object -First 1
        if ($Msi) {
            Copy-Item $Msi.FullName "$Stage/drzdiff_${Version}_x64.msi" -Force
        }
    }
    Pop-Location

    # Install scripts
    @"
@echo off
REM install.bat - DRZ Diff $Version
setlocal
pushd "%~dp0"

if exist "drzdiff_${Version}_x64.msi" goto :msi_install

echo --- .msi not found, installing via direct copy ---
set "INSTALLDIR=%LOCALAPPDATA%\Programs\DRZ Diff"
if not exist "%INSTALLDIR%" mkdir "%INSTALLDIR%"
copy /Y "drzdiff.exe" "%INSTALLDIR%\drzdiff.exe" >NUL
echo.
echo Installed DRZ Diff $Version to %INSTALLDIR%.
echo To create a Start Menu shortcut, run install_shortcut.ps1.
popd
endlocal
exit /b 0

:msi_install
echo Installing DRZ Diff $Version via .msi...
msiexec /i "drzdiff_${Version}_x64.msi" /qb! ADDLOCAL=ALL
if errorlevel 1 ( echo Install failed.& popd & exit /b 1 )
echo Done. Find "DRZ Diff" in Start Menu.
popd
endlocal
exit /b 0
"@ | Set-Content "$Stage/install.bat" -Encoding ASCII

    @"
`$ErrorActionPreference = 'Stop'
`$InstallDir = Join-Path `$env:LOCALAPPDATA 'Programs\DRZ Diff'
`$Exe = Join-Path `$InstallDir 'drzdiff.exe'
if (-not (Test-Path `$Exe)) { Write-Error "drzdiff.exe not found at `$Exe — run install.bat first"; exit 1 }
`$ShortcutDir = Join-Path `$env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
if (-not (Test-Path `$ShortcutDir)) { New-Item -ItemType Directory -Path `$ShortcutDir | Out-Null }
`$ShortcutPath = Join-Path `$ShortcutDir 'DRZ Diff.lnk'
`$Shell = New-Object -ComObject WScript.Shell
`$Shortcut = `$Shell.CreateShortcut(`$ShortcutPath)
`$Shortcut.TargetPath = `$Exe
`$Shortcut.WorkingDirectory = `$InstallDir
`$Shortcut.IconLocation = "`$(`$Exe),0"
`$Shortcut.Save()
Write-Host "Created Start Menu shortcut: `$ShortcutPath"
"@ | Set-Content "$Stage/install_shortcut.ps1" -Encoding ASCII

    @"
@echo off
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
"@ | Set-Content "$Stage/uninstall.bat" -Encoding ASCII

    # SHA256SUMS
    Push-Location $Stage
    Get-ChildItem -File | Where-Object { $_.Name -ne 'SHA256SUMS' } | ForEach-Object {
        $hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower()
        "{0}  {1}" -f $hash,$_.Name
    } | Set-Content 'SHA256SUMS' -Encoding ASCII
    Pop-Location

    Get-ChildItem $Stage
    Write-Host "[windows] done"
}
finally {
    Pop-Location
}
