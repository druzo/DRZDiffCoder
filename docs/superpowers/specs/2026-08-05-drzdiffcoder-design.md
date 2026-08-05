# DRZDiffCoder — Design Spec

**Date:** 2026-08-05
**Stack:** Rust + egui/eframe + tree-sitter + ropey + similar
**Architecture:** MVVM
**License:** GPL
**Platforms:** Linux, Windows, macOS — single codebase

## 1. Goal

Desktop source-code diff/merge app:

- File-to-file compare (side-by-side)
- Editable panes, chunk-copy merge
- Git difftool/mergetool integration
- **Incremental multilanguage syntax highlighting** (tree-sitter, true incremental reparse)

## 2. Architecture — MVVM

```
┌─────────────────────────────────────────────┐
│ View (egui) — drz-app, drz-diff-ui,         │
│ drz-editor widget. Renders VM state each    │
│ frame (immediate mode pull). Input events → │
│ VM commands. NO model access, NO logic.     │
└──────────────┬──────────────────────────────┘
               │ commands (edit, open, merge, save)
               ▼
┌─────────────────────────────────────────────┐
│ ViewModel — drz-viewmodel.                  │
│ AppViewModel / DiffViewModel /              │
│ EditorViewModel. Holds UI state, exposes    │
│ commands, owns Model handles. No egui       │
│ imports. Async results (diff thread) arrive │
│ via channel → applied to state. Headless-   │
│ testable.                                   │
└──────────────┬──────────────────────────────┘
               │ reads/mutates
               ▼
┌─────────────────────────────────────────────┐
│ Model — drz-core (Document: rope, path,     │
│ dirty flag; DiffResult: hunks),             │
│ drz-highlight (tree-sitter engine = model-  │
│ side service). Pure data + domain logic.    │
└─────────────────────────────────────────────┘
```

### MVVM rules

1. View imports ViewModel only. Never touches rope/tree-sitter/diff types directly.
2. ViewModel never imports egui/eframe. Repaint signaling via `Arc<dyn Fn()>` callback injected by app shell (egui `Context::request_repaint` wired there).
3. Immediate mode = pull binding: each frame View reads VM state snapshot. No observer framework needed.
4. Async work (diff, file load): VM spawns thread, result returns through `std::sync::mpsc` channel; VM drains channel at frame start.
5. All merge/edit/highlight logic lives in VM+Model → fully unit-testable headless, no GUI harness required.

### Crate structure (cargo workspace)

| Crate | MVVM layer | Purpose | Key deps |
|---|---|---|---|
| `drz-core` | Model | Rope text buffer, diff engine, file I/O, encoding | ropey, similar, chardetng |
| `drz-highlight` | Model (service) | tree-sitter engine, grammar registry, incremental reparse, highlight queries → colored byte ranges | tree-sitter, grammar crates |
| `drz-viewmodel` | ViewModel | EditorViewModel, DiffViewModel, AppViewModel; commands, state, async channels | drz-core, drz-highlight |
| `drz-editor` | View | egui code-editor widget: gutter, per-line `LayoutJob` cache render, input → commands | egui, drz-viewmodel |
| `drz-diff-ui` | View | Synced panes, change gutter, connector lines, chunk-copy arrows | egui, drz-viewmodel |
| `drz-app` | View (shell) | Window, menus, dialogs, CLI entry; wires VM↔View | eframe, clap, rfd |

## 3. Key decisions

- **ropey** rope buffer → O(log n) edits; every edit emits `tree_sitter::InputEdit` → tree-sitter reparses only dirty subtree. Incremental end-to-end.
- **Highlight cache:** tree-sitter ranges → per-line `LayoutJob`; only dirty lines rebuilt. Rendering never re-highlights whole file.
- **similar** crate for diff (Myers + histogram). No hand-rolled diff.
- **Git v1 = CLI args only.** Works as `git difftool` (`drzdiff $LOCAL $REMOTE`) and mergetool. No libgit2 in v1.
- **Custom editor widget** — `egui_code_editor`/syntect rejected (regex-based, not structural/incremental).
- Diff recompute on background thread, debounced ~150ms.

## 4. Phasing

1. **Phase 1 (MVP):** 2-file compare, editable panes, incremental highlight, save, difftool CLI mode, scroll sync, connector lines.
2. **Phase 2:** 3-way merge (base/local/remote → result), mergetool mode.
3. **Phase 3:** repo browsing (git status/commit diffs); libgit2-vs-CLI decided then.

## 5. Data flow

**Load:** read file → encoding detect (UTF-8 default, chardetng fallback) → rope → language detect (extension map → tree-sitter grammar; unknown = plain text) → full tree-sitter parse once → highlight cache → diff (similar, line-level) → render both panes + change map.

**Keystroke hot path:**

```
key → rope edit (O(log n))
    → InputEdit {start, old_end, new_end} → tree.edit()
    → parser.parse(old_tree)  // re-parses only dirty subtree
    → re-query changed range → rebuild LayoutJob for dirty lines only
    → mark diff dirty → recompute diff async (debounced ~150ms)
    → repaint
```

**Scroll sync:** both panes share scroll offset model; alignment computed from diff hunks (equal blocks align, changed blocks padded with virtual blank lines); connector polygon drawn between panes.

**Merge action:** click chunk arrow → apply inverse diff hunk to other pane's rope → same incremental pipeline re-runs.

## 6. Error handling

- File read fail (perms/binary/too large) → error panel, no crash. Binary detection (NUL in first 8KB) → refuse with message. Size cap configurable (default 50MB; above = plain text, diff only).
- Encoding errors → lossy decode + banner "encoding guessed".
- tree-sitter grammar missing/parse timeout → degrade to plain text, log warning. Per-language grammar load = lazy + fallible.
- Diff thread panic → caught via `join`, show stale-diff marker, never poison UI.
- Save conflict (file changed on disk) → warn + offer reload/overwrite.
- No `unwrap`/`expect` in app code; `anyhow::Result` app layer, `thiserror` library crates. `panic = abort` never set.

## 7. Testing

- **drz-core:** unit tests — rope edit invariants, diff hunks vs `git diff` output on fixture corpus, encoding edge cases.
- **drz-highlight:** golden tests — fixture file per language → snapshot colored ranges (insta crate). Incremental test: apply edit, assert only expected ranges invalidated; assert dirty-reparse tree equals full reparse tree.
- **drz-viewmodel:** headless integration tests — drive commands (open docs, edit, merge chunk, save), assert state transitions. No egui. Main logic-test surface.
- **drz-editor / drz-diff-ui:** pure-math unit tests (cursor math, line cache, hunk alignment). Screenshot tests deferred (egui kittest if needed later).
- **Integration:** CLI difftool invocation test — spawn binary with two files, assert exit code + no panic (assert_cmd).
- **CI:** GitHub Actions matrix linux/windows/macos, `cargo test` + clippy + fmt.

## 8. Out of scope (v1)

Folder/directory compare, image/hex diff, plugin system, themes beyond light/dark, i18n.
