# DRZDiffCoder — Agent Notes

Rust source-code diff/compare tool. egui/eframe + tree-sitter + ropey + similar. MVVM.

## Layout (cargo workspace)

| Crate | Layer | Purpose |
|---|---|---|
| `drz-core` | Model | `Rope` buffer, `diff_lines`, `TextEdit`, file I/O, encoding |
| `drz-highlight` | Model | tree-sitter engine, incremental reparse, `LanguageId` |
| `drz-viewmodel` | ViewModel | `AppViewModel`, `DiffViewModel`, `EditorViewModel` — **no egui imports** |
| `drz-editor` | View | `CodeEditor` widget (gutter, styled lines, cursor) |
| `drz-diff-ui` | View | Side-by-side synced panes, center strip, merge arrows |
| `drz-app` | View shell | eframe window, CLI entry, theme, icons, `.desktop` install |

Binary: `drzdiff` (`crates/drz-app/src/main.rs`). CLI: `drzdiff <left> <right>`.

## Commands

```bash
cargo build --workspace
cargo test --workspace
cargo test -p <crate>                 # e.g. drz-core, drz-viewmodel
cargo test -p drz-core diff           # single test name
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

CI runs those three checks in order: `fmt --check` → `clippy -D warnings` → `test`. Fix in reverse.

## Hard rules (MVVM + project)

- `drz-viewmodel` MUST NOT depend on `egui`/`eframe`. Repaint callback injected at app shell.
- View crates MUST NOT touch `ropey`/`tree-sitter`/`similar` directly — only via `drz-viewmodel` re-exports.
- `drz-app` uses `anyhow::Result`; library crates use `thiserror`.
- No `unwrap`/`expect` in non-test code.
- File-size cap: 50MB default. Binary = NUL byte in first 8KB → refuse with message.
- Diff recompute on background thread, debounced ~150ms.
- Each edit to a `Document` emits exactly one `tree_sitter::InputEdit` → incremental reparse.
- Languages (syntactic): Rust, Python, JS/JSX, C, C++. Anything else → plain text.

## Conventions

- Conventional Commits: `feat:`, `fix:`, `test:`, `chore:`, `docs:`, `ci:`. One task = one commit.
- Branch flow: `main` ← `develop` ← feature branches. Current work: `feature/UXEnchancements`.
- PRs target `develop`. CI runs on `main` push + all PRs.
- Spec: `docs/superpowers/specs/2026-08-05-drzdiffcoder-design.md`.
- Plan: `docs/superpowers/plans/2026-08-05-drzdiffcoder-mvp.md`.

## Repo quirks

- **Icon assets use `include_bytes!("../../../icons/…")`** — keep that nesting (5 levels deep). Moving icons under `src/` breaks the build.
- **Linux only:** `icon::install_desktop_integration()` writes `~/.local/share/applications/drzdiff.desktop` + icon sizes. Wayland ignores `with_icon` — needs the `.desktop` entry. Failures silently ignored.
- **CLI smoke test** (`crates/drz-app/tests/cli.rs`) **skips when `DISPLAY` unset AND `CI` set.** Exercise locally: `DISPLAY=:0 cargo test -p drz-app`.
- **No `rustfmt.toml` / `clippy.toml`** — defaults apply. Run `cargo fmt --all` before committing.
- **Default theme = dark** (DrzApp). User toggle persisted in `eframe::Storage` under key `drz_theme_dark`.
- **No screenshot tests** (Phase 1). Headless VM tests in `drz-viewmodel` are the main logic surface.

## Where to look first

- New feature? Read `docs/superpowers/specs/...-design.md` §5 (data flow) before touching code.
- Bug in editor? `drz-editor/src/editor.rs` + `drz-diff-ui/src/diff_view.rs`.
- Diff math? `drz-core/src/{diff,align,inline}.rs`.
- Tree-sitter? `drz-highlight/src/engine.rs`.
- Async / commands? `drz-viewmodel/src/{app_vm,diff_vm,editor_vm}.rs`.

## Don't

- Don't add `cargo` deps without checking the workspace `[workspace.dependencies]` block first.
- Don't touch `crates/drz-viewmodel/src/*` from view crates.
- Don't reach for `syntect`/`egui_code_editor` — explicitly rejected in spec §3.
- Don't introduce `unsafe` — tree-sitter/ropey/similar/egui are all safe.
- Don't `.unwrap()` file reads / edits — propagate errors via `CoreError` / `anyhow`.
