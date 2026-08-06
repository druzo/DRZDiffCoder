# Editor Selection, Clipboard, and Context Menu

Date: 2026-08-06
Status: Approved
Branch: `feature/UXEnchancements`
Related: `docs/superpowers/specs/2026-08-05-drzdiffcoder-design.md`

## 1. Goal

Add text selection (mouse + keyboard) to `CodeEditor`, wire OS-default Copy / Cut / Paste shortcuts, and add a right-click context menu with icons. The menu items enable only when the action is valid (Copy/Cut require a non-empty selection; Paste requires the system clipboard to hold text).

This work unblocks the side-by-side diff view (`drz-diff-ui`) automatically, since both panes reuse `CodeEditor`.

## 2. Non-goals

- Undo/redo (separate spec).
- Find/replace.
- Multi-cursor.
- Drag-and-drop text between left and right panes.
- Shortcut remapping UI.
- Screenshot tests (forbidden per `AGENTS.md`).

## 3. User-visible behavior

### Selection triggers

| Input | Result |
|---|---|
| Left click | Collapse selection to caret. |
| Left click + drag | Extend selection; anchor at click, cursor follows pointer. |
| Right click | Open the context menu (does NOT modify the selection). |
| Shift + click | Extend selection from existing anchor. |
| Shift + Arrow (← → ↑ ↓) | Extend selection by one col (or one line, keeping col). |
| Double-click | Select the word under the cursor (alphanumeric + `_`). A click on whitespace snaps to the nearest word char before selection. |
| Triple-click | Select the current line (start col 0 to end of line, inclusive of trailing `\n` if present). |
| Ctrl/Cmd + A | Select entire document. |

Selection is rendered as a translucent overlay over the selected byte range, distinct from added/removed row backgrounds. The overlay is drawn after row backgrounds but before text, so it remains visible against added/removed tints. Per-line byte ranges for multi-line selections are computed by `selection_per_line_range`, which returns `(col_start, col_end)` for each row inside the selection and `None` for rows outside.

Word boundary: scan left/right from the click byte-col with `char_indices()`; contiguous runs of `[A-Za-z0-9_]` count as one word. Non-matching characters (operators, whitespace, punctuation, multi-byte CJK) are word breaks.

### Clipboard shortcuts

| Shortcut | Behavior |
|---|---|
| Ctrl/Cmd + C | Copy selected text to system clipboard. No-op if selection empty. |
| Ctrl/Cmd + X | Cut: copy selection to clipboard, replace selection with `""`, caret moves to selection start. |
| Ctrl/Cmd + V | Paste: replace selection (or insert at caret if no selection) with clipboard text. No-op if clipboard empty. |
| Backspace / Delete | With selection: replace selection with `""` (same path as cut, no clipboard). |
| Typing | With selection: replace selection with typed char, caret advances. |

### Context menu

Right-click in the editor opens a vertical menu with:

| Item | Icon | Enabled when |
|---|---|---|
| Copy | `doc.on.clipboard.svg` | selection is non-empty |
| Cut | `scissors.svg` | selection is non-empty |
| Paste | `doc.on.clipboard.fill.svg` | system clipboard holds text |
| Select All | (no icon) | always |

Items are rendered with the icon on the left, label on the right, both vertically centered. Disabled items render dimmed and do not respond to clicks.

## 4. Architecture

### State placement

- **Selection state** (`Selection { anchor: (usize, usize), cursor: (usize, usize) }`) lives in `drz-viewmodel` as a shared type. Methods: `Selection::ordered() -> ((usize, usize), (usize, usize))`, `is_selected()`, `collapse()`.
- **Selection instance** (the per-pane anchor/cursor, plus drag tracking) lives on `CodeEditor` in `drz-editor`. `None` selection = caret only; `Some` selection = active range.
- **Buffer operations** live on `EditorViewModel` in `drz-viewmodel`. View crates must never touch `ropey` / `tree-sitter` / `similar` directly per `AGENTS.md`.

### MVVM boundary

- `drz-viewmodel` exposes:
  - `text_in_range(start: (usize, usize), end: (usize, usize)) -> String`
  - `replace_selection_with(start: (usize, usize), end: (usize, usize), new_text: &str) -> usize` — returns new caret col after insert; caret placement is the VM's contract (line stays the same as `start.0`, col = `start.1 + new_text.len()`).
  - Both methods route through existing `edit()` so exactly one `tree_sitter::InputEdit` reaches `drz-highlight` per call (per `AGENTS.md` hard rule).
- `drz-editor` owns interaction (mouse drag, double-click timing, context menu rendering, keyboard shortcut dispatch). It calls VM for read/write of selection text.
- `drz-diff-ui`, `drz-app` are unchanged. New behavior surfaces automatically because both panes reuse `CodeEditor`.

### Icons

Three SVG files copied from `/home/druzo/Desenvolvimento/ApplicationIcons/AppleIcons/` into the project `icons/` directory (root), preserving the existing `include_bytes!("../../../icons/…")` 5-level nesting:

- `doc.on.clipboard.svg` (Copy)
- `scissors.svg` (Cut)
- `doc.on.clipboard.fill.svg` (Paste)

Rasterization at runtime via `resvg` + `usvg` + `tiny-skia` (new workspace deps). Decoded into `egui::TextureHandle`s, 14×14 px, cached on a new `EditorIcons` struct held by `CodeEditor`. Fallback: text glyph (`Copy`, `Cut`, `Paste`) if rasterization errors.

## 5. Data flow

### Selection read (Copy / context-menu Copy / Ctrl+C)

```
1. CodeEditor reads self.selection (Some/None).
2. If Some, get ordered start/end.
3. vm.text_in_range(start, end) → String.
4. ui.ctx().copy_text(s) writes to system clipboard.
5. Selection unchanged.
```

### Selection cut (Ctrl+X / context-menu Cut)

```
1. Compute ordered start/end.
2. s = vm.text_in_range(start, end).
3. ui.ctx().copy_text(s).
4. vm.replace_selection_with(start, end, "").
5. self.selection = None (collapsed caret).
6. self.cursor = (start.0, start.1).
```

### Selection paste (Ctrl+V / context-menu Paste)

```
1. If self.selection.is_some() → start, end from it.
   Else → start = end = self.cursor.
2. text = ui.ctx().clipboard_text() (egui 0.27+; otherwise read once and cache).
3. If text empty → no-op.
4. new_col = vm.replace_selection_with(start, end, text).
5. self.selection = None.
6. self.cursor.0 = start.0; self.cursor.1 = new_col.
```

### Typing / Backspace / Delete with selection

```
1. ordered start, end = self.selection.ordered().
2. Delete the range via replace_selection_with(start, end, "").
3. Apply the new input:
   - Plain char: insert_at_line_col(start.0, start.1, ch); caret = (start.0, start.1 + ch.len()).
   - Enter: insert "\n"; caret = (start.0 + 1, 0).
   - Backspace/Delete: no further action; caret = start.
4. self.selection = None.
```

### Mouse drag

```
1. response.drag_started() → set self.selection_anchor = click pos; cursor = anchor.
2. While response.dragged():
   pos = response.interact_pointer_pos()
   self.cursor = (clamped line, clamped col)
3. response.drag_stopped() → keep selection; clear drag tracking.
```

### Double-click / triple-click

- `response.double_clicked()` triggers word selection: scan left/right from click byte-col with `char_indices()` over `vm.line(line)`; set `selection = Some((line, left_byte), (line, right_byte))`.
- Triple-click: track `last_double_click_at: Option<Instant>`. If a double-click fires within 300 ms of the previous double-click AND on the same line, treat as triple-click → set `selection = Some((line, 0), (line, line_byte_len))`. Triple-click supersedes the double-click.

### UTF-8 safety

All byte-col operations pass through `floor_col_boundary()` (already in `crates/drz-editor/src/editor.rs`) before any rope slice / insert. Same guard applies to selection endpoints.

## 6. Files

### New

| Path | Purpose |
|---|---|
| `icons/doc.on.clipboard.svg` | Copy icon (from `ApplicationIcons/AppleIcons/`). |
| `icons/scissors.svg` | Cut icon. |
| `icons/doc.on.clipboard.fill.svg` | Paste icon. |
| `crates/drz-editor/src/icon.rs` | `EditorIcons` struct: lazy SVG → `TextureHandle` cache via `resvg`. Methods `copy(&self) -> Option<&TextureHandle>`, `cut`, `paste`. |

### Modified

| Path | Change |
|---|---|
| `Cargo.toml` (workspace) | Add `resvg`, `usvg`, `tiny-skia` under `[workspace.dependencies]`. |
| `crates/drz-editor/Cargo.toml` | Add the three SVG deps. |
| `crates/drz-editor/src/lib.rs` | `pub mod icon;`. |
| `crates/drz-editor/src/editor.rs` | Add `Selection` import from viewmodel; selection state field; drag / double / triple-click handling in `show()`; context menu via `response.context_menu(...)`; extend `handle_keys` for Shift+arrow, Ctrl+A/C/X/V, selection-aware typing/Backspace/Delete; init `icons: EditorIcons` field; pass icons into context menu rendering. |
| `crates/drz-viewmodel/src/lib.rs` | `pub use editor_vm::Selection;` (new shared type). |
| `crates/drz-viewmodel/src/editor_vm.rs` | Add `pub struct Selection { anchor, cursor }` with `ordered`, `is_selected`, `collapse`; add `text_in_range`, `replace_selection_with`. |

### Unchanged

`drz-core`, `drz-highlight`, `drz-diff-ui`, `drz-app`. The diff view picks up the new behavior because both panes share `CodeEditor`.

## 7. Testing

### Viewmodel headless (no egui required)

- `text_in_range` on single-line, multi-line, empty doc, reversed endpoints (anchor > cursor returns text for ordered pair), UTF-8 boundary slicing.
- `replace_selection_with` round-trip: select → replace → text correct, `edit_seq` bumps by exactly 1, single `tree_sitter::InputEdit` emitted (re-use existing `rust_edit_keeps_highlight_in_sync` pattern, routed through the new method).
- `Selection::ordered/is_selected/collapse` pure-logic tests.

### Editor unit

- Word-boundary scanner: input `"foo bar_baz.qux 42"`, click at col 5 → expected range `(0, 7)` ("foo bar"); click at col 8 → `(8, 15)` ("bar_baz"); click at col 16 → `(16, 19)` ("qux"); click at col 20 → `(20, 22)` ("42").
- Triple-click detection timing threshold (300 ms, same line).
- `Selection::ordered` swaps when `anchor > cursor` bytewise.

### Manual smoke (developer machine)

```
cargo run -p drz-app -- crates/drz-editor/tests/fixtures/left.rs crates/drz-editor/tests/fixtures/right.rs
# then in the GUI:
# 1. Drag-select a word in either pane; confirm highlight.
# 2. Right-click; Copy / Cut enabled, Paste enabled (clipboard has text from step 1).
# 3. Right-click an empty area; Copy / Cut disabled (dimmed).
# 4. Ctrl+C in editor, paste into another app — text matches.
# 5. Ctrl+V from another app — text appears at caret, replacing any selection.
# 6. Double-click word; triple-click line; Shift+arrow extend.
# 7. Ctrl+A selects all; typing replaces selection.
```

### CI verification (per `AGENTS.md`)

```
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Fix in reverse order if any step fails.

## 8. Risks

- **egui clipboard read API**: `egui::Context::clipboard_text` / `clipboard_has_text` exist in egui 0.27+. If the workspace is on an older egui, fall back to reading via `arboard` crate (already a transitive dep of `eframe` on desktop). If neither available, log a warning and disable the Paste path. The Copy/Cut paths always work (`copy_text` is the long-standing API).
- **Selection through padding rows** (side-by-side diff): when a click lands on a display row where `line_of_row` returns `None`, clamp the resolved line to `vm.len_lines().saturating_sub(1)` as `editor.rs` already does. Selection endpoints never resolve to a padding line.
- **Empty document**: `Ctrl+A` selects from `(0,0)` to `(0,0)`; Copy / Cut are no-ops; Paste inserts at `(0,0)`.
- **Drag race with focus**: a drag that begins on an unfocused widget steals focus (matches VS Code / GNOME Text Editor behavior).

## 9. Out of scope (recorded for later specs)

- Undo/redo.
- Find/replace.
- Multi-cursor / multi-selection.
- Drag-and-drop text between left and right panes (would need a `merge_chunk`-style integration).
- Keyboard shortcut remapping / settings UI.
- Touch / mobile gestures.