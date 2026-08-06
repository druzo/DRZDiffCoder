<p align="center">
  <img src="icons/App big Icon.png" alt="DRZ Diff" width="220"/>
</p>

<h1 align="center">DRZ Diff</h1>

<p align="center">
  <strong>Side-by-side source-code diff with true incremental syntax highlighting.</strong><br>
  22 languages · 5 platforms · GPL-3.0 · single self-contained binary.
</p>

<p align="center">
  <a href="releases/0.1.0/"><img alt="release" src="https://img.shields.io/badge/release-v0.1.0-blueviolet?style=for-the-badge"></a>
  <a href="LICENSE"><img alt="license" src="https://img.shields.io/badge/license-GPL--3.0-blue?style=for-the-badge"></a>
  <a href="#supported-languages"><img alt="languages" src="https://img.shields.io/badge/languages-22-success?style=for-the-badge"></a>
  <a href="#installation"><img alt="platforms" src="https://img.shields.io/badge/platforms-windows%20%C2%B7%20linux%20%C2%B7%20macOS-lightgrey?style=for-the-badge"></a>
  <a href="https://www.rust-lang.org"><img alt="rust" src="https://img.shields.io/badge/rust-1.95%2B-orange?style=for-the-badge"></a>
</p>

---

> **TL;DR.** DRZ Diff is a fast, offline, language-aware file-diff and merge tool that uses `tree-sitter` for true incremental re-parsing — typing one character on a 10,000-line file re-parses only the affected sub-tree. It ships as a single self-contained binary for Windows, Linux, and macOS with **zero runtime dependencies** on the `.AppImage` / `.dmg` / `.exe` paths.

---

## ✨ Features at a Glance

| | Feature |
|---|---|
| 🧠 **True incremental parsing** | `tree-sitter` re-parses only the dirty sub-tree; the editor stays responsive on multi-MB files. |
| 🎨 **22 languages** | Rust, Python, JS/JSX, C, C++, Java, C#, SQL, R, Pascal, Go, Assembly, PHP, Kotlin, Dart, Lua, Julia, Lisp, Scala, Objective-C, Swift, JSON. Plain-text fallback for the rest. |
| 🔀 **Side-by-side diff + merge** | Synced panes, change-band center strip, click-to-copy merge arrows, char-level inline emphasis. |
| ✏️ **Editable panes** | Click-drag / double-click (word) / triple-click (line) selection, `Ctrl+C/X/V`, right-click context menu with icons. |
| ⚡ **Async diff** | Background thread, ~150 ms debounce, channel-fed result drain. UI never freezes. |
| 💾 **Smart file handling** | UTF-8 + `chardetng` fallback, binary detection (NUL in first 8 KB → refuse), 50 MB cap. |
| 🎭 **Theme toggle** | Dark default, light optional, persisted in `eframe::Storage`. |
| 🪟 **Native shell** | CLI entry (`drzdiff <left> <right>`), `git difftool`/`git mergetool` friendly, drag-and-drop. |
| 📦 **Five installers** | `.msi` (Win) · `.dmg` + `.app` (macOS ×2) · `.deb` + `.AppImage` (Linux x86_64) · `.deb` (Linux arm64). |

---

## 🏗️ Architecture

DRZ Diff is a strict **MVVM** Rust application organised as a cargo workspace. The view layer is pure `egui`, the view-model layer has zero `egui` imports, the model layer has zero `eframe` awareness, and async results flow through `std::sync::mpsc` channels drained at frame start.

```
┌─────────────────────────────────────────────────────────────┐
│  View (egui)         drz-app · drz-diff-ui · drz-editor     │
│  ─────────────────  Renders VM state each frame (immediate  │
│                      mode pull). Input events → commands.   │
│                      NO model access, NO logic.             │
└──────────────────────────────┬──────────────────────────────┘
                               │ commands (edit, open, merge, save)
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  ViewModel            drz-viewmodel                          │
│  ─────────────────  AppViewModel · DiffViewModel ·         │
│                      EditorViewModel. Owns Model handles,  │
│                      async results arrive via channel,      │
│                      NO egui imports. Headless-testable.    │
└──────────────────────────────┬──────────────────────────────┘
                               │ reads / mutates
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  Model                drz-core (Rope, Diff, file I/O)        │
│                      drz-highlight (tree-sitter service)    │
│  ─────────────────  Pure data + domain logic.              │
└─────────────────────────────────────────────────────────────┘
```

### Crate structure

| Crate | MVVM layer | Purpose | Lines of Rust |
|---|---|---|---:|
| `drz-core` | Model | `Rope` buffer, `diff_lines`, `TextEdit`, file I/O, encoding | ~1.5 k |
| `drz-highlight` | Model (service) | tree-sitter engine, incremental reparse, `LanguageId` | ~1.3 k |
| `drz-viewmodel` | ViewModel | `AppViewModel`, `DiffViewModel`, `EditorViewModel` — **no egui imports** | ~1.4 k |
| `drz-editor` | View | `CodeEditor` widget (gutter, styled lines, cursor) | ~0.7 k |
| `drz-diff-ui` | View | Side-by-side synced panes, change bands, merge arrows | ~0.6 k |
| `drz-app` | Shell | eframe window, CLI entry, theme, icons, `.desktop` install | ~1.2 k |

### Data flow on every keystroke

```
key → rope edit (O(log n))
    → InputEdit {start, old_end, new_end} → tree.edit()
    → parser.parse(old_tree)   // only dirty sub-tree
    → re-query changed range   → rebuild LayoutJob for dirty lines only
    → mark diff dirty → recompute diff async (debounced ~150 ms)
    → repaint
```

Scroll sync: both panes share a scroll-offset model; alignment is computed from diff hunks — equal blocks align, changed blocks are padded with virtual blank rows.

---

## 📦 Installation

> **Download** the pre-built artifacts from [`releases/0.1.0/`](releases/0.1.0/) and pick your platform. Every artifact ships with a per-folder `SHA256SUMS` — verify with `sha256sum -c SHA256SUMS`.

### 🪟 Windows

**Prerequisites:** Windows 10 or newer (x86_64).

**Option A — Installer (`.msi`, built on CI):**

1. Download `drzdiff_0.1.0_x64.msi` from `releases/0.1.0/windows-x86_64/`.
2. Double-click the `.msi`. Windows SmartScreen may warn — click *More info → Run anyway* (unsigned binaries).
3. Find **DRZ Diff** in the Start Menu.

**Option B — Direct copy (`install.bat` fallback):**

1. Download the `windows-x86_64/` folder.
2. Double-click `install.bat`. It will copy `drzdiff.exe` to `%LOCALAPPDATA%\Programs\DRZ Diff\`.
3. Optionally run `install_shortcut.ps1` (right-click → *Run with PowerShell*) to add a Start Menu shortcut.

**Verify:**
```powershell
"%LOCALAPPDATA%\Programs\DRZ Diff\drzdiff.exe" --help
```
You should see:
```
DRZ Diff — source code diff tool
Usage: drzdiff [LEFT] [RIGHT]
```

---

### 🐧 Linux

**Prerequisites:** glibc 2.31+ (for the AppImage) or any Debian/Ubuntu derivative (for the `.deb`).

#### x86_64

**Option A — Debian package (recommended):**
```bash
cd linux-x86_64
sudo dpkg -i drzdiff_0.1.0_amd64.deb
sudo apt-get install -f -y   # resolve any missing system libs
drzdiff
```

**Option B — AppImage (universal):**
```bash
cd linux-x86_64
chmod +x drzdiff-0.1.0-x86_64.AppImage
./drzdiff-0.1.0-x86_64.AppImage
```

Or use the helper script:
```bash
./install.sh   # picks .deb (with sudo) or AppImage automatically
```

#### aarch64 (Pi 5, Asahi, Graviton, …)

```bash
cd linux-arm64
sudo dpkg -i drzdiff_0.1.0_arm64.deb
sudo apt-get install -f -y
drzdiff
```

**Verify:**
```bash
drzdiff --help
```

> **Wayland tip:** On first run, `drzdiff` self-installs a `.desktop` entry and `drzdiff.png` icons into `~/.local/share/icons/hicolor/` so the taskbar/dock icon resolves. No manual steps required.

---

### 🍎 macOS

**Prerequisites:** macOS 11 Big Sur or newer.

**Option A — Disk image (`.dmg`):**
```bash
cd darwin-x86_64   # or darwin-arm64 on Apple Silicon
bash install.sh    # mounts the .dmg and copies DRZDiff.app to /Applications
```

**Option B — Direct copy from the `.app` bundle:**
```bash
cd darwin-x86_64
cp -R DRZDiff.app /Applications/
xattr -dr com.apple.quarantine /Applications/DRZDiff.app
open /Applications/DRZDiff.app
```

**Manual install:** Drag `DRZDiff.app` from the mounted `.dmg` to `/Applications/`.

**Verify:**
```bash
/Applications/DRZDiff.app/Contents/MacOS/DRZDiff --help
```

> **Note:** The bundle is *ad-hoc* signed (`codesign --sign -`) — Gatekeeper will warn on first launch. Right-click → *Open* to bypass, or run `xattr -dr com.apple.quarantine /Applications/DRZDiff.app`.

---

## 🚀 Quick Start

### CLI (git difftool-friendly)

```bash
drzdiff path/to/left.rs path/to/right.rs
```

### As a `git difftool`

```bash
git config --global diff.tool drzdiff
git config --global difftool.drzdiff.cmd 'drzdiff "$LOCAL" "$REMOTE"'
git difftool
```

### As a `git mergetool` (Phase 2)

```bash
git config --global merge.tool drzdiff
git config --global mergetool.drzdiff.cmd 'drzdiff "$LOCAL" "$REMOTE" "$BASE" "$MERGED"'
git mergetool
```

### GUI

1. Launch `drzdiff` (or `drzdiff.exe` / `DRZDiff.app`).
2. The **Welcome screen** appears — drag two files onto the window, click **Open files**, or pass paths on the command line.
3. The panes open side-by-side with synchronized scrolling.
4. Click a **merge arrow** in the center strip to copy a hunk from one side to the other.
5. Edit any line — the diff recomputes ~150 ms after you stop typing.

---

## 💡 Usage

### Opening files

- **Welcome screen**: drag-and-drop two files anywhere on the window.
- **Toolbar**: *Open…* (or `Ctrl+O`) → pick the left and right files.
- **CLI**: `drzdiff <left> <right>`.
- **Swap sides**: click the swap icon in the toolbar to flip left ↔ right.

### Editing

| Action | Shortcut / gesture |
|---|---|
| Click | Position cursor |
| Double-click | Select word |
| Triple-click | Select line |
| Click + drag | Select range |
| `Shift` + arrows | Extend selection |
| `Shift` + `Home` / `End` | Extend to start / end of line |
| `Ctrl+A` | Select all |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | Copy / Cut / Paste |
| Right-click | Context menu (Cut / Copy / Paste / Select All) |
| `Ctrl+Z` / `Ctrl+Y` | *(Phase 2)* |
| `Esc` | Clear selection |

### Merging

- Click an arrow button in the **center strip** next to a changed hunk to copy that hunk from one pane to the other.
- The receiving pane updates immediately; the diff recomputes asynchronously.

### Saving

- `Ctrl+S` saves the currently focused pane.
- If the file changed on disk since it was opened, you'll be prompted to *reload* or *overwrite*.

### Themes

- Toolbar → ☀ / 🌙 toggle.
- Preference is persisted in `eframe::Storage` under key `drz_theme_dark`.

---

## 🌍 Supported Languages

| Language | Extensions | Grammar |
|---|---|---|
| Rust | `.rs` | `tree-sitter-rust` |
| Python | `.py`, `.pyi` | `tree-sitter-python` |
| JavaScript | `.js`, `.mjs`, `.cjs`, `.jsx` | `tree-sitter-javascript` |
| C | `.c`, `.h` | `tree-sitter-c` |
| C++ | `.cpp`, `.cc`, `.cxx`, `.hpp`, `.hh` | `tree-sitter-cpp` |
| Java | `.java` | `tree-sitter-java` |
| C# | `.cs`, `.csx` | `tree-sitter-c-sharp` |
| SQL | `.sql` | `tree-sitter-sequel` |
| R | `.R`, `.r` | `tree-sitter-r` |
| Delphi / Object Pascal | `.pas`, `.pp`, `.dpr`, `.dpk` | `tree-sitter-pascal` |
| Go | `.go` | `tree-sitter-go` |
| Assembly | `.asm`, `.s`, `.S` | `tree-sitter-asm` |
| PHP | `.php`, `.phtml`, `.php5` | `tree-sitter-php` |
| Kotlin | `.kt`, `.kts` | `tree-sitter-kotlin-ng` |
| Dart | `.dart` | `tree-sitter-dart` |
| Lua | `.lua` | `tree-sitter-lua` |
| Julia | `.jl` | `tree-sitter-julia` |
| Common Lisp | `.lisp`, `.cl`, `.clj`, `.scm`, `.el` | `tree-sitter-commonlisp` |
| Scala | `.scala`, `.sc` | `tree-sitter-scala` |
| Objective-C | `.m`, `.mm` | `tree-sitter-objc` |
| Swift | `.swift` | `tree-sitter-swift` |
| JSON | `.json`, `.jsonc`, `.json5` | `tree-sitter-json` |
| Plain text | *anything else* | (no grammar, no highlighting) |

Language detection is extension-based; unknown extensions fall back to plain text without errors.

---

## 🎨 Theming

- **Dark mode** (default) — magenta-on-navy brand bar, lime merge-arrow accent, cyan toolbar accents.
- **Light mode** — same hue palette with inverted backgrounds, tuned for long reading sessions.
- The choice is persisted across launches via `eframe::Storage`.

---

## 🧪 Development

### Building from source

**Prerequisites:**
- **Rust** ≥ 1.95 (stable) — install via [rustup](https://rustup.rs)
- **C toolchain** for tree-sitter native bindings (gcc/clang + make)
- **Platform packages:**
  - Linux: `build-essential` `pkg-config` `libssl-dev`
  - macOS: Xcode Command Line Tools (`xcode-select --install`)
  - Windows: MSVC Build Tools or MinGW

**Clone & build:**
```bash
git clone https://github.com/druzo/DRZDiffCoder.git
cd DRZDiffCoder
cargo build --workspace --release
./target/release/drzdiff path/to/left.rs path/to/right.rs
```

### Common commands

```bash
cargo build --workspace                    # debug build
cargo build --workspace --release          # release build
cargo test --workspace                     # run all tests (142 tests)
cargo test -p drz-core diff                # run a single test
cargo test --workspace --quiet             # only print failures
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

### MVVM hard rules

These are enforced by code review and CI:

1. **`drz-viewmodel` MUST NOT depend on `egui` / `eframe`.** Repaint signalling uses an `Arc<dyn Fn()>` callback injected at the app shell.
2. **View crates MUST NOT touch `ropey` / `tree-sitter` / `similar` directly.** Always go through `drz-viewmodel` re-exports.
3. **`drz-app` uses `anyhow::Result`; library crates use `thiserror`.**
4. **No `unwrap()` / `expect()` in non-test code.**
5. **File-size cap: 50 MB default.** Binary = NUL byte in first 8 KB → refuse with a message.
6. **Diff recompute runs on a background thread, debounced ~150 ms.**
7. **Every edit to a `Document` emits exactly one `tree_sitter::InputEdit` → incremental reparse.**

### Conventional Commits

```
feat: …   new feature
fix:  …   bug fix
docs: …   documentation only
chore: …  tooling / non-functional
test: …   tests
ci:   …   CI / CD
refactor: …  internal restructuring (no behaviour change)
```

Branch flow: `main` ← `develop` ← `feature/*`. PRs target `develop`.

---

## 🛠️ Building Release Artifacts

Reproduce the `releases/0.1.0/` folder from this checkout:

```bash
# one-time host setup
sudo ./scripts/release/install-prereqs.sh

# build all platforms
./scripts/release.sh
```

Or pick specific platforms:

```bash
VERSION=0.1.0 PLATFORMS="windows linux-x86_64 linux-arm64 darwin-x86_64 darwin-arm64" \
  ./scripts/release.sh
```

The pipeline is **idempotent** — re-running with the same `VERSION` overwrites prior artifacts.

CI wires this up via `.github/workflows/release.yml`: push a tag like `v0.1.0` and the workflow builds every platform, attaches all installers to a GitHub Release, and uploads `SHA256SUMS`.

---

## 🐛 Troubleshooting

<details>
<summary><strong>"Binary file detected" — refusing to open</strong></summary>

DRZ Diff refuses to diff files containing a NUL byte in the first 8 KB (binary heuristic). To compare binary files, use a hex diff (`xxd | diff`) instead.

</details>

<details>
<summary><strong>"File too large" (>50 MB)</strong></summary>

Default size cap is 50 MB. Files above this are opened as plain text, no diff. Reduce the file size or open a subset.

</details>

<details>
<summary><strong>On Linux: "libgtk-3-0 not found"</strong></summary>

Install dependencies: `sudo apt install libgtk-3-0 libxcb-render0 libxcb-shape0 libxcb-xfixes0 libdbus-1-3 libatk1.0-0 libatk-bridge2.0-0 libxkbcommon0 libatspi2.0-0`

Or use the AppImage, which is self-contained.

</details>

<details>
<summary><strong>On macOS: "DRZDiff.app is damaged"</strong></summary>

The bundle is ad-hoc signed but not notarized. Strip the quarantine attribute:

```bash
xattr -dr com.apple.quarantine /Applications/DRZDiff.app
```

Or right-click → *Open* the first time to bypass Gatekeeper.

</details>

<details>
<summary><strong>On Windows: SmartScreen warning</strong></summary>

Binaries are unsigned for v0.1.0. Click *More info → Run anyway*. Production deployments should sign with an Authenticode certificate.

</details>

<details>
<summary><strong>AppImage won't launch ("FUSE not available")</strong></summary>

Install FUSE: `sudo apt install libfuse2t64 fuse3`. Alternatively, extract the AppImage with `--appimage-extract-and-run` and run `./squashfs-root/AppRun`.

</details>

<details>
<summary><strong>Wayland taskbar shows generic icon</strong></summary>

On first launch, DRZ Diff self-installs a `.desktop` entry and the `drzdiff.png` icon into `~/.local/share/icons/hicolor/`. If the icon still doesn't show, run:

```bash
gtk-update-icon-cache -q ~/.local/share/icons/hicolor
update-desktop-database ~/.local/share/applications
```

…and re-log.

</details>

<details>
<summary><strong>Build fails: "aarch64-linux-gnu-gcc not found"</strong></summary>

Install the cross-compiler: `sudo apt-get install gcc-aarch64-linux-gnu g++-aarch64-linux-gnu`

</details>

<details>
<summary><strong>Build fails: "x86_64-w64-mingw32-gcc not found"</strong></summary>

Install: `sudo apt-get install gcc-mingw-w64-x86-64`

</details>

---

## 🤝 Contributing

1. Fork & clone.
2. Create a branch off `develop`: `git checkout -b feature/my-thing`
3. Make your change with Conventional Commits (`feat:`, `fix:`, …).
4. Add tests for new logic in `drz-viewmodel` or `drz-core` — these are the headless logic surfaces.
5. Run before pushing:
   ```bash
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
6. Open a PR against `develop`.

See [`AGENTS.md`](AGENTS.md) for the full agent notes (crate layout, hard rules, conventions).

---

## 📜 License

**GPL-3.0** — see [`LICENSE`](LICENSE).

```
DRZ Diff — source code diff/compare tool
Copyright (C) 2026 DRZ
```

---

## 📚 Further Reading

- [Design spec](docs/superpowers/specs/2026-08-05-drzdiffcoder-design.md) — MVVM architecture, crate structure, data flow.
- [MVP implementation plan](docs/superpowers/plans/2026-08-05-drzdiffcoder-mvp.md).
- [Selection + clipboard + context-menu spec](docs/superpowers/specs/2026-08-06-editor-selection-clipboard-design.md).
- [Release-pipeline design](docs/superpowers/specs/2026-08-06-release-pipeline-design.md).
- [Release notes for v0.1.0](releases/0.1.0/RELEASE-NOTES.md).

---

<p align="center">
  Built with 🦀 Rust · 🪟 egui · 🌳 tree-sitter · 📜 ropey · 🔀 similar
</p>
