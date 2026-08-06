# Release pipeline — implementation plan

Date: 2026-08-06
Spec: `docs/superpowers/specs/2026-08-06-release-pipeline-design.md`

## Tasks

1. **Toolchain install (one-time, long)** — apt + manual downloads. Bg it.
   - `sudo apt-get install -y gcc-mingw-w64-x86-64 dpkg-dev liblzma-dev libfuse2 fuse3 libarchive-tools fakeroot`
   - `cargo install cargo-wix --locked` (Windows installer)
   - `cargo install winresource --locked` (Windows icon embed; needed for the cargo dep to compile)
   - Download `osxcross` SDK tarball + build osxcross (clones `tpoechtrager/osxcross`, downloads `MacOSX15.5.sdk.tar.xz` from a hosted source) — this is the slowest step
   - Download `appimagetool-x86_64.AppImage` from AppImage GitHub release
   - Install `libdmg-hfsplus` from source: `git clone https://github.com/fanquake/libdmg-hfsplus && cmake + make` → provides `hfs/hfsplus` tools
   - Install `png2icns` (used by make-icons) or use ImageMagick `magick` for everything
   - `rustup target add x86_64-pc-windows-gnu aarch64-unknown-linux-gnu x86_64-apple-darwin aarch64-apple-darwin`

2. **Cargo change** (`crates/drz-app/`)
   - Add `winresource = "0.1"` under `[workspace.dependencies]` in `Cargo.toml`
   - In `crates/drz-app/Cargo.toml`, add:
     ```toml
     [target.'cfg(target_os = "windows")'.build-dependencies]
     winresource.workspace = true
     ```
   - Create `crates/drz-app/build.rs`:
     ```rust
     #[cfg(target_os = "windows")]
     fn main() {
         winresource::WindowsResource::new()
             .set_icon("icons/AppIcon.ico")
             .expect("set_icon")
             .compile()
             .expect("compile resources");
     }
     #[cfg(not(target_os = "windows"))]
     fn main() {}
     ```
   - Verify `cargo build -p drz-app` still works on Linux host (build.rs noop there).

3. **`scripts/release/make-icons.sh`** — idempotent. Generates:
   - `icons/AppIcon.ico` — multi-size (16,32,48,64,128,256) via `magick AppIcon.png -define icon:auto-resize=256,128,96,64,48,32,16 AppIcon.ico`
   - `icons/AppIcon.icns` — generate `iconset` dir with PNGs at 16/32/64/128/256/512 @1x+@2x then `iconutil -c icns icons/AppIcon.iconset`

4. **`scripts/release/check.sh`** — pure bash. Verifies each tool exists, prints MISSING block + exit 1 if any. Tools to check:
   `cargo rustup dpkg-deb fakeroot magick convert wine x86_64-w64-mingw32-gcc aarch64-linux-gnu-gcc o64-clang aarch64-apple-darwin21.4-clang hfs dmg appimagetool cargo-wix winresource_icon` (iconutil via osxcross PATH)

5. **`scripts/release/build-windows.sh`**
   - `cargo build --release --target x86_64-pc-windows-gnu -p drz-app`
   - Stage `target/x86_64-pc-windows-gnu/release/drzdiff.exe` to staging
   - Run `cargo wix init` once (produces `wix/main.wxs`), edit to add product description + icon
   - `cargo wix --no-build` → produces `target/wix/drzdiff-<VER>-x86_64.msi`
   - Copy `.msi` + `.exe` to `releases/<VER>/windows-x86_64/`
   - Write `install.bat` + `uninstall.bat` from template
   - `sha256sum * > SHA256SUMS`

6. **`scripts/release/build-macos.sh`** — accepts `aarch64|x86_64`
   - Source osxcross env: `export PATH="$OSXCROSS/target/bin:$PATH"`
   - `cargo build --release --target <triple>-apple-darwin -p drz-app`
   - Assemble `DRZDiff.app/Contents/{MacOS,Resources,Info.plist,icons.iconset}`
   - Copy `target/.../drzdiff` → `DRZDiff.app/Contents/MacOS/DRZDiff`
   - Copy `icons/AppIcon.icns` → `DRZDiff.app/Contents/Resources/AppIcon.icns`
   - Hand-write `Info.plist` with `CFBundleIdentifier=app.drzdiff`, `CFBundleName=DRZ Diff`, `CFBundleIconFile=AppIcon.icns`, `CFBundleExecutable=DRZDiff`, `LSMinimumSystemVersion=11.0`
   - `hfsplus` create sparse image → `hfsplus` write `DRZDiff.app` → `dmg` build
   - Stage to `releases/<VER>/darwin-<arch>/`
   - Write `install.sh` (uses `hdiutil attach` flow but actually since target is Linux, install.sh just prints manual instruction + uses `cp` after mount; this is what the user will run on a Mac)

7. **`scripts/release/build-linux.sh`**
   - `cargo build --release --target <triple>-unknown-linux-gnu -p drz-app`
   - For each target:
     - **amd64**: also build AppImage (`linuxdeploy` AppImage or hand-rolled AppDir + `appimagetool`)
     - Both arches: build `.deb` (assemble `debian/` tree with `control`, `postinst`, `prerm`, `data.tar.xz`, control archive)
   - Stage + `SHA256SUMS`

8. **`scripts/release.sh`** orchestrator — env-var `VERSION`, calls each in sequence with progress banners. Tracks `target/` to keep cross-builds incremental.

9. **`.github/workflows/release.yml`** — `ubuntu-24.04`, single job, `actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, install prereqs via shell script, run `release.sh`, upload via `softprops/action-gh-release@v2` with `files: releases/<VER>/**/*`.

10. **`.gitignore`** — append `/releases/`.

11. **Run pipeline**. Capture stderr if anything fails.

12. **Verify**:
    - `ls -R releases/0.1.0/` shows 5 platform dirs
    - `file $(find releases/0.1.0 -type f -executable | head)` shows correct arches
    - `sha256sum -c releases/0.1.0/*/SHA256SUMS` passes
    - `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` stays green

13. **Commit** spec + plan + scripts + workflow (NOT `releases/` since ignored).

## Risks / fallbacks

- **osxcross SDK** may fail to download if Apple moves hosting. Fallback: pin to a specific commit of `tpoechtrager/osxcross` known to work.
- **libdmg-hfsplus** may fail to compile on newer glibc. Fallback: use `genisoimage` to build a UDRW image + manual `cp` (less polished but works).
- **appimagetool** download may require newer FUSE. Fallback: bundle `--appimage-extract-and-run` or use `--appimage` form with `--no-fuse`.
- **AppImage for arm64** is harder (requires QEMU). Out of scope per spec; arm64 gets `.deb` only.
- **build.rs path for .ico**: `build.rs` runs in `crates/drz-app/`, so the relative path `icons/AppIcon.ico` resolves correctly. Icon generation must happen BEFORE `cargo build` — `release.sh` orders `make-icons.sh` first.

## Order of execution (cronological)

1. Kick off toolchain installs (parallel, can take 30+ min)
2. While installing, write all source files (cargo, build.rs, scripts, workflow)
3. When installs finish, run `release.sh`
4. Verify artifacts
5. Commit source changes (spec + scripts + workflow)
