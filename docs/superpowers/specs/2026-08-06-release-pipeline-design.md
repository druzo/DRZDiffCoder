# Release pipeline — cross-platform artifacts

Date: 2026-08-06
Status: approved
Branch: `feature/UXEnchancements`

## Goal

From one command (`./scripts/release.sh`) produce installable artifacts for:

| OS | Arch | Artifacts | Icon source |
|---|---|---|---|
| Windows | x86_64 | `drzdiff.exe`, `.msi` (WiX), `install.bat`, `uninstall.bat` | `.ico` embedded at build (PE) |
| macOS | x86_64 + arm64 | `DRZDiff.app`, `.dmg`, `install.sh` | `.icns` in `Resources/` |
| Linux | x86_64 | `drzdiff`, `.deb`, `.AppImage`, `install.sh` | PNG in hicolor + desktop file |
| Linux | aarch64 | `drzdiff`, `.deb`, `install.sh` | PNG in hicolor + desktop file |

Each release lives in `releases/<VERSION>/` (5 platform subfolders + per-platform `SHA256SUMS`).

## Version source

Read `VERSION` from env → fall back to git tag `v*` → fall back to `git rev-parse --short HEAD`. Abort if it disagrees with `crates/drz-app/Cargo.toml` `[package].version`. Folder name strips the leading `v`.

## Icon embedding strategy

- **Linux** keeps the existing `include_bytes!("../../../icons/AppIcon.png")` for the runtime window icon; `.deb` + AppImage also drop `drzdiff.png` into `/usr/share/icons/hicolor/<N>x<N>/apps/` so the desktop file resolves.
- **Windows** adds a `build.rs` to `drz-app`, gated `cfg(windows)`. It calls `winresource::WindowsResource::new().icon("icons/AppIcon.ico")` to embed the icon in the PE resource table so File Explorer shows it. `.ico` is generated from `AppIcon.png` by `scripts/release/make-icons.sh` at the start of every release.
- **macOS** generates `AppIcon.icns` via `iconutil` + a prebuilt `iconset` from PNG, drops it into `DRZDiff.app/Contents/Resources/`, sets `CFBundleIconFile=AppIcon.icns` and `CFBundleIconName=AppIcon` in `Info.plist`. Done at packaging time by `scripts/release/build-macos.sh`.

## File layout (new + modified)

```
crates/drz-app/
  Cargo.toml            # +winresource target-gated
  build.rs              # NEW (windows-only)
scripts/
  release.sh            # NEW entry point
  release/
    check.sh            # NEW prereq validator
    make-icons.sh       # NEW png→ico/icns
    build-windows.sh    # NEW cargo + cargo-wix
    build-macos.sh      # NEW cargo + osxcross + hdiutil + libdmg
    build-linux.sh      # NEW cargo + dpkg-deb + appimagetool
  installers/
    install.bat.in      # NEW (templated)
    uninstall.bat.in    # NEW
    install.sh.in       # NEW (templated per platform)
build-aux/
  wix/
    drzdiff.wxs.in      # NEW WiX template
  linux/
    postinst.in         # NEW dpkg postinst
    prerm.in            # NEW dpkg prerm
    control.in          # NEW deb control
github/workflows/release.yml   # NEW
.gitignore              # +/releases
```

## Pipeline flow (`scripts/release.sh`)

1. Resolve `VERSION`.
2. `scripts/release/check.sh` — verify each prereq. Exit non-zero if missing (with actionable message listing the missing tool + install hint).
3. `scripts/release/make-icons.sh` — idempotent. Regenerates `icons/AppIcon.ico` + `icons/AppIcon.icns` from `AppIcon.png`.
4. For each platform:
   - `cargo build --release --target <triple>` (shared `target/`).
   - Bundle installer.
   - Stage artifacts into `releases/$VERSION/<platform>/`.
   - Generate `SHA256SUMS`.
   - Copy/install `install.{bat,sh}` from template.
5. Emit per-platform summary table (sizes + checksums).
6. CI: `softprops/action-gh-release@v2` uploads everything.

## Constraints preserved

- MVVM boundaries untouched. Only `drz-app` gains a windows-only `build.rs` + target-gated dep.
- No `unsafe`.
- New shell scripts use `set -euo pipefail`.
- Pipeline is idempotent: re-running same `VERSION` overwrites prior artifacts (folder is wiped at step 4 for each platform).
- `releases/` ignored by git; CI uploads directly to a GitHub Release.

## Out of scope (this iteration)

- **Windows .msi when building on Linux+Wine**: WiX `light.exe` fails with `LGHT0216 / Win32 0x65B` — known incompatibility between Wine and the cabinet extractor used by WiX 3.x. The pipeline still calls `cargo wix`; if it fails (currently always, on Linux+Wine) the build logs a warning and falls back to a direct-copy `install.bat`. CI on a Windows runner does build the .msi cleanly — the same `scripts/release/build-windows.sh` is used.
- **macOS .dmg**: requires the osxcross toolchain + a downloaded Apple SDK (linked from `osxcross/build_sdk.sh`). The SDK is freely available but requires accepting Apple's EULA. On hosts without the SDK, the pipeline skips macOS targets silently (logged as `MISSING: osxcross` in check).
- Code signing (Windows EV cert, Apple Developer ID notarization, GPG key).
- Auto-update / Sparkle / WinSquirrel.
- `rpmbuild` for Fedora/RHEL — apt installs don't provide it.
- Universal/fat macOS binary (lipo) — `.app` per arch is enough for now; user picks the right `.dmg`.

## Verification

`./scripts/release.sh` exits 0. `releases/<VER>/` contains 5 platform folders, every folder has a binary + ≥1 installer + `SHA256SUMS`. `file` output for each binary matches its target triple. `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` stays green after the `drz-app` changes.
