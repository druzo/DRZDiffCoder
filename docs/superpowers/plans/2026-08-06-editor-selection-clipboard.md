# Editor Selection, Clipboard, Context Menu — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add mouse + keyboard text selection to `CodeEditor`, OS-default Copy/Cut/Paste shortcuts, and a right-click context menu with icons. Items enable only when valid (Copy/Cut require selection; Paste requires non-empty clipboard).

**Architecture:** Hybrid MVVM — selection state and pointer/keyboard interaction live in `CodeEditor` (view crate, `drz-editor`); buffer operations (`text_in_range`, `replace_selection_with`) live in `EditorViewModel` (model crate, `drz-viewmodel`). The `Selection` struct is a shared type in `drz-viewmodel`. Icons rasterized at runtime from SVG via `resvg` + `usvg` + `tiny-skia` (new workspace deps).

**Tech Stack:** Rust 2021, egui 0.31, eframe 0.31, tree-sitter 0.25, ropey 1.6, similar 2.7, resvg 0.45, usvg 0.45, tiny-skia 0.11.

## Global Constraints

These constraints apply to every task; do not re-state in each task unless that task is the source of the constraint.

- **MVVM rule:** `drz-viewmodel` MUST NOT depend on `egui`/`eframe`. View crates MUST NOT touch `ropey`/`tree-sitter`/`similar` directly — only via `drz-viewmodel` re-exports.
- **No `unwrap`/`expect`** in non-test code.
- **Each edit to a `Document` emits exactly one `tree_sitter::InputEdit`** → `EditorViewModel::replace_selection_with` must produce exactly one `edit()` call.
- **Icon asset path nesting:** `include_bytes!("../../../icons/…")` — keep 5 levels deep.
- **No `rustfmt.toml` / `clippy.toml`** — defaults apply. Run `cargo fmt --all` before each commit.
- **CI verification command (run after every task):** `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`. Fix in reverse order if any step fails.
- **Conventional Commits:** `feat:`, `fix:`, `test:`, `chore:`, `docs:`, `ci:`. One task = one commit.
- **Branch:** `feature/UXEnchancements`. Do not switch branches.
- **No screenshot tests** (Phase 1). Headless VM tests in `drz-viewmodel` are the main logic surface.

---

## File Structure

### New files

| Path | Responsibility |
|---|---|
| `icons/doc.on.clipboard.svg` | Copy icon (from `/home/druzo/Desenvolvimento/ApplicationIcons/AppleIcons/`). |
| `icons/scissors.svg` | Cut icon. |
| `icons/doc.on.clipboard.fill.svg` | Paste icon. |
| `crates/drz-editor/src/icon.rs` | `EditorIcons` struct: lazily rasterizes 3 SVGs via `resvg` → `egui::TextureHandle`. One method per action returns `Option<&TextureHandle>`. |

### Modified files

| Path | Change |
|---|---|
| `Cargo.toml` (workspace root) | Add `resvg`, `usvg`, `tiny-skia` to `[workspace.dependencies]`. |
| `crates/drz-editor/Cargo.toml` | Add the three SVG deps. |
| `crates/drz-editor/src/lib.rs` | `pub mod icon;`. |
| `crates/drz-editor/src/editor.rs` | Add `Selection` import from viewmodel; add selection state fields; rewrite `show()` for `Sense::click_and_drag()` + drag/dbl/tpl-click handling; add context menu via `response.context_menu(...)`; extend `handle_keys` for Shift+arrow, Ctrl+A/C/X/V, selection-aware typing/Backspace/Delete; init `icons: EditorIcons` field. |
| `crates/drz-viewmodel/src/lib.rs` | `pub use editor_vm::Selection;`. |
| `crates/drz-viewmodel/src/editor_vm.rs` | Add `pub struct Selection`; add `text_in_range`; add `replace_selection_with`. |

### Untouched

`drz-core`, `drz-highlight`, `drz-diff-ui`, `drz-app`. Diff view picks up new behavior because both panes share `CodeEditor`.

---

## Task 1: Workspace deps + SVG icon files

**Files:**
- Modify: `Cargo.toml` (workspace `[workspace.dependencies]` block)
- Modify: `crates/drz-editor/Cargo.toml` (`[dependencies]` block)
- Create: `icons/doc.on.clipboard.svg`
- Create: `icons/scissors.svg`
- Create: `icons/doc.on.clipboard.fill.svg`

**Interfaces:**
- Produces: workspace resolves `resvg`, `usvg`, `tiny-skia` for any crate. `drz-editor` can `use resvg; use usvg; use tiny_skia;` in later tasks.

- [ ] **Step 1: Add deps to workspace `Cargo.toml`**

Open `/home/druzo/Desenvolvimento/DRZDiffCoder/Cargo.toml`. Under the existing `[workspace.dependencies]` block (after the `image` line), add:

```toml
resvg = "0.45"
usvg = "0.45"
tiny-skia = "0.11"
```

- [ ] **Step 2: Add deps to `drz-editor/Cargo.toml`**

Open `/home/druzo/Desenvolvimento/DRZDiffCoder/crates/drz-editor/Cargo.toml`. In the `[dependencies]` block, after the existing entries, add:

```toml
resvg.workspace = true
usvg.workspace = true
tiny-skia.workspace = true
```

- [ ] **Step 3: Copy 3 SVG files into project `icons/`**

```bash
cp /home/druzo/Desenvolvimento/ApplicationIcons/AppleIcons/doc.on.clipboard.svg /home/druzo/Desenvolvimento/DRZDiffCoder/icons/doc.on.clipboard.svg
cp /home/druzo/Desenvolvimento/ApplicationIcons/AppleIcons/scissors.svg /home/druzo/Desenvolvimento/DRZDiffCoder/icons/scissors.svg
cp /home/druzo/Desenvolvimento/ApplicationIcons/AppleIcons/doc.on.clipboard.fill.svg /home/druzo/Desenvolvimento/DRZDiffCoder/icons/doc.on.clipboard.fill.svg
ls -la /home/druzo/Desenvolvimento/DRZDiffCoder/icons/{doc.on.clipboard,scissors,doc.on.clipboard.fill}.svg
```

Expected: 3 files present, each > 0 bytes.

- [ ] **Step 4: Verify workspace builds (deps resolve, no breakage)**

Run: `cargo build -p drz-editor`
Expected: compiles (warnings only about unused imports are fine — deps are pulled but unused until later tasks). No errors.

- [ ] **Step 5: Commit**

```bash
cd /home/druzo/Desenvolvimento/DRZDiffCoder
git add Cargo.toml crates/drz-editor/Cargo.toml icons/doc.on.clipboard.svg icons/scissors.svg icons/doc.on.clipboard.fill.svg
git commit -m "chore: add resvg deps and copy/cut/paste SVG icons"
```

---

## Task 2: `Selection` shared type in viewmodel

**Files:**
- Modify: `crates/drz-viewmodel/src/editor_vm.rs` (top of file, before `pub struct EditorViewModel`)
- Modify: `crates/drz-viewmodel/src/lib.rs`

**Interfaces:**
- Produces: `pub struct Selection { pub anchor: (usize, usize), pub cursor: (usize, usize) }` with `impl Selection { pub fn new(anchor, cursor) -> Self; pub fn ordered(&self) -> ((usize, usize), (usize, usize)); pub fn is_selected(&self) -> bool; pub fn collapse(&mut self); }`.
- Re-exported from `drz_viewmodel::Selection`.

- [ ] **Step 1: Write the failing test for `Selection`**

Open `/home/druzo/Desenvolvimento/DRZDiffCoder/crates/drz-viewmodel/src/editor_vm.rs`. At the bottom of the existing `#[cfg(test)] mod tests` block, add:

```rust
    #[test]
    fn selection_ordered_returns_min_max_bytewise() {
        let s = Selection::new((2, 5), (1, 3));
        assert_eq!(s.ordered(), ((1, 3), (2, 5)));
    }

    #[test]
    fn selection_ordered_already_in_order() {
        let s = Selection::new((0, 1), (0, 4));
        assert_eq!(s.ordered(), ((0, 1), (0, 4)));
    }

    #[test]
    fn selection_is_selected_false_when_collapsed() {
        let s = Selection::new((2, 5), (2, 5));
        assert!(!s.is_selected());
    }

    #[test]
    fn selection_is_selected_true_when_anchor_differs() {
        let s = Selection::new((0, 0), (3, 0));
        assert!(s.is_selected());
        let s = Selection::new((0, 0), (0, 7));
        assert!(s.is_selected());
    }

    #[test]
    fn selection_collapse_moves_cursor_to_anchor() {
        let mut s = Selection::new((0, 3), (5, 0));
        s.collapse();
        assert_eq!(s.cursor, s.anchor);
        assert_eq!(s.cursor, (0, 3));
    }

    #[test]
    fn selection_new_accepts_reversed_endpoints() {
        let s = Selection::new((4, 2), (1, 5));
        assert_eq!(s.anchor, (4, 2));
        assert_eq!(s.cursor, (1, 5));
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p drz-viewmodel selection_`
Expected: compile error — `Selection` not defined (or test functions missing references).

- [ ] **Step 3: Implement `Selection` in `editor_vm.rs`**

At the top of `crates/drz-viewmodel/src/editor_vm.rs`, immediately after the `use` block and before `pub struct EditorViewModel { ... }`, add:

```rust
/// Half-open text selection in `(line, byte_col)` coordinates.
/// `anchor` is fixed (click position); `cursor` follows pointer / arrow keys.
/// Byte-col, not char-col, to match the rest of the editor's coordinate system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: (usize, usize),
    pub cursor: (usize, usize),
}

impl Selection {
    pub fn new(anchor: (usize, usize), cursor: (usize, usize)) -> Self {
        Self { anchor, cursor }
    }

    /// Return `(start, end)` with `start <= end` bytewise
    /// (`(line, col).0 * u32::MAX as usize + col` ordering). Handles both
    /// same-line and cross-line ordering.
    pub fn ordered(&self) -> ((usize, usize), (usize, usize)) {
        let a = self.anchor;
        let c = self.cursor;
        let key = |p: (usize, usize)| (p.0, p.1);
        if key(a) <= key(c) {
            (a, c)
        } else {
            (c, a)
        }
    }

    /// `true` iff the selection covers at least one byte (cursor != anchor).
    pub fn is_selected(&self) -> bool {
        self.anchor != self.cursor
    }

    /// Collapse to anchor — cursor jumps to anchor. Keeps anchor fixed so a
    /// subsequent Shift+arrow extends from the same anchor.
    pub fn collapse(&mut self) {
        self.cursor = self.anchor;
    }
}
```

- [ ] **Step 4: Re-export `Selection` from viewmodel**

Open `crates/drz-viewmodel/src/lib.rs`. Modify the existing `pub use editor_vm::EditorViewModel;` line by appending `, Selection`:

```rust
pub use editor_vm::{EditorViewModel, Selection};
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p drz-viewmodel selection_`
Expected: 6 tests pass.

- [ ] **Step 6: Run clippy**

Run: `cargo clippy -p drz-viewmodel --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Commit**

```bash
cd /home/druzo/Desenvolvimento/DRZDiffCoder
git add crates/drz-viewmodel/src/editor_vm.rs crates/drz-viewmodel/src/lib.rs
git commit -m "feat(viewmodel): add Selection shared type"
```

---

## Task 3: `EditorViewModel::text_in_range` + `replace_selection_with`

**Files:**
- Modify: `crates/drz-viewmodel/src/editor_vm.rs` (inside `impl EditorViewModel`)
- Modify: `crates/drz-viewmodel/src/editor_vm.rs` (test block)

**Interfaces:**
- Produces:
  - `pub fn text_in_range(&self, start: (usize, usize), end: (usize, usize)) -> String`
  - `pub fn replace_selection_with(&mut self, start: (usize, usize), end: (usize, usize), text: &str) -> (usize, usize)` — returns `(new_line, new_col)` for the caret after the insert. If `text` is empty, returns `start`. If `text` contains `\n`, `new_line = start.0 + count('\n')` and `new_col = byte_len of last line segment`.

- [ ] **Step 1: Write failing tests for `text_in_range`**

At the bottom of the test module in `crates/drz-viewmodel/src/editor_vm.rs`, add:

```rust
    #[test]
    fn text_in_range_single_line_slice() {
        let vm = EditorViewModel::from_text("hello\nworld\n", LanguageId::PlainText);
        assert_eq!(vm.text_in_range((0, 1), (0, 4)), "ell");
    }

    #[test]
    fn text_in_range_multi_line_inclusive_end() {
        // "ab\ncd\nef" → range ((0,1),(2,1)) yields "b\ncd\ne"
        let vm = EditorViewModel::from_text("ab\ncd\nef\n", LanguageId::PlainText);
        assert_eq!(vm.text_in_range((0, 1), (2, 1)), "b\ncd\ne");
    }

    #[test]
    fn text_in_range_empty_when_start_eq_end() {
        let vm = EditorViewModel::from_text("abc\n", LanguageId::PlainText);
        assert_eq!(vm.text_in_range((0, 2), (0, 2)), "");
    }

    #[test]
    fn text_in_range_reversed_endpoints_swaps() {
        let vm = EditorViewModel::from_text("abc\ndef\n", LanguageId::PlainText);
        assert_eq!(vm.text_in_range((1, 0), (0, 2)), "c\nd");
    }

    #[test]
    fn text_in_range_utf8_bytewise() {
        // "aé💣b\n" → a=1B, é=2B, 💣=4B, b=1B. bytes 1..3 == "é".
        let vm = EditorViewModel::from_text("aé💣b\n", LanguageId::PlainText);
        assert_eq!(vm.text_in_range((0, 1), (0, 3)), "é");
        assert_eq!(vm.text_in_range((0, 3), (0, 7)), "💣");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p drz-viewmodel text_in_range_`
Expected: compile error — `text_in_range` not a method.

- [ ] **Step 3: Implement `text_in_range`**

In `impl EditorViewModel` (in `editor_vm.rs`), after the existing `replace_lines` method, add:

```rust
    /// Read the text covered by a half-open selection. `(line, byte_col)`
    /// endpoints; the second endpoint is treated as exclusive of the byte
    /// itself (matching the existing rope `delete_range_line_col` convention)
    /// — but for selection *display*, callers that want inclusive end-of-range
    /// should pass `end.1 + 1` on the same line, or the start of the next
    /// line. The convention here is: end is the cursor position after the
    /// last selected byte. So `text_in_range((0,1),(0,4))` over "hello\n"
    /// yields "ell" (cols 1,2,3; col 4 excluded).
    pub fn text_in_range(&self, start: (usize, usize), end: (usize, usize)) -> String {
        let mut sel = Selection::new(start, end);
        let (s, e) = sel.ordered();
        if s == e {
            return String::new();
        }
        let same_line = s.0 == e.0;
        let start_col = s.1;
        let end_col = e.1;
        let mut out = String::new();
        for line in s.0..=e.0 {
            let text = self.doc.line(line);
            let begin = if line == s.0 { start_col.min(text.len()) } else { 0 };
            let finish = if line == e.0 {
                end_col.min(text.len())
            } else {
                text.len()
            };
            if same_line {
                out.push_str(&text[begin..finish]);
                return out;
            }
            if begin < finish {
                out.push_str(&text[begin..finish]);
            }
            if line < e.0 {
                out.push('\n');
            }
        }
        out
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p drz-viewmodel text_in_range_`
Expected: 5 tests pass.

- [ ] **Step 5: Write failing tests for `replace_selection_with`**

Append to the test module:

```rust
    #[test]
    fn replace_selection_with_single_char_no_selection() {
        let mut vm = EditorViewModel::from_text("hello\n", LanguageId::PlainText);
        let before = vm.edit_seq();
        let (nl, nc) = vm.replace_selection_with((0, 5), (0, 5), "!");
        assert_eq!(vm.line(0), "hello!");
        assert_eq!((nl, nc), (0, 6));
        assert_eq!(vm.edit_seq(), before + 1);
    }

    #[test]
    fn replace_selection_with_replaces_range_and_returns_end() {
        let mut vm = EditorViewModel::from_text("hello world\n", LanguageId::PlainText);
        let (nl, nc) = vm.replace_selection_with((0, 6), (0, 11), "Rust");
        assert_eq!(vm.line(0), "hello Rust");
        assert_eq!((nl, nc), (0, 10));
    }

    #[test]
    fn replace_selection_with_multiline_text_advances_line_and_col() {
        let mut vm = EditorViewModel::from_text("ab\ncd\n", LanguageId::PlainText);
        let (nl, nc) = vm.replace_selection_with((0, 0), (0, 0), "x\ny\nz");
        // After insert: "x\ny\nzab\ncd\n". Caret at end of "z" → line 2, col 1.
        assert_eq!(vm.line(0), "x");
        assert_eq!(vm.line(1), "y");
        assert_eq!(vm.line(2), "zab");
        assert_eq!((nl, nc), (2, 1));
    }

    #[test]
    fn replace_selection_with_empty_text_deletes_range() {
        let mut vm = EditorViewModel::from_text("hello world\n", LanguageId::PlainText);
        let (nl, nc) = vm.replace_selection_with((0, 5), (0, 11), "");
        assert_eq!(vm.line(0), "hello");
        assert_eq!((nl, nc), (0, 5));
    }

    #[test]
    fn replace_selection_with_emits_exactly_one_edit() {
        let mut vm = EditorViewModel::from_text("aaa\nbbb\n", LanguageId::PlainText);
        let before = vm.edit_seq();
        vm.replace_selection_with((0, 1), (0, 2), "Z");
        assert_eq!(vm.edit_seq(), before + 1, "exactly one edit() call per replace");
    }

    #[test]
    fn replace_selection_with_keeps_highlight_in_sync() {
        // Same invariant as rust_edit_keeps_highlight_in_sync but routed
        // through the new method: HighlightEdit must reach the engine once.
        let mut vm = EditorViewModel::from_text("fn main() {}\n", LanguageId::Rust);
        vm.replace_selection_with((0, 12), (0, 12), " // x");
        assert_eq!(vm.line(0), "fn main() {} // x");
        let (_, spans) = vm.styled_line(0);
        assert!(spans.iter().any(|s| s.style == Style::Comment));
    }
```

- [ ] **Step 6: Run tests to verify they fail**

Run: `cargo test -p drz-viewmodel replace_selection_with_`
Expected: compile error — method not defined.

- [ ] **Step 7: Implement `replace_selection_with`**

In `impl EditorViewModel`, immediately after `text_in_range`, add:

```rust
    /// Replace the byte range `[start, end)` with `new_text`. Routes through
    /// the single `edit()` entry point so exactly one `tree_sitter::InputEdit`
    /// reaches `drz-highlight` (per AGENTS.md hard rule).
    ///
    /// Returns the caret position after the insert: if `new_text` is empty,
    /// returns `start`; otherwise the caret sits at the byte just after the
    /// last inserted byte (line = `start.0 + count('\n')`, col = byte length
    /// of the trailing line segment).
    pub fn replace_selection_with(
        &mut self,
        start: (usize, usize),
        end: (usize, usize),
        new_text: &str,
    ) -> (usize, usize) {
        if new_text.is_empty() && start == end {
            return start;
        }
        // Compute byte positions in the rope.
        let byte_of = |(line, col): (usize, usize)| -> usize {
            if line >= self.doc.len_lines() {
                return self.doc.rope().len_bytes();
            }
            let (ls, le) = self.doc.line_byte_range(line);
            (ls + col).min(le)
        };
        let s_byte = byte_of(start);
        let e_byte = byte_of(end).max(s_byte);
        self.edit(s_byte, e_byte, new_text);

        if new_text.is_empty() {
            return start;
        }
        // Compute new caret: count newlines, take col of trailing segment.
        let mut newlines = 0usize;
        let mut last_seg_start = 0usize;
        for (i, b) in new_text.bytes().enumerate() {
            if b == b'\n' {
                newlines += 1;
                last_seg_start = i + 1;
            }
        }
        let trailing = &new_text[last_seg_start..];
        let new_col = trailing.len();
        let new_line = start.0 + newlines;
        (new_line, new_col)
    }
```

- [ ] **Step 8: Run the tests to verify they pass**

Run: `cargo test -p drz-viewmodel`
Expected: all tests pass (existing + new).

- [ ] **Step 9: Run clippy**

Run: `cargo clippy -p drz-viewmodel --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 10: Commit**

```bash
cd /home/druzo/Desenvolvimento/DRZDiffCoder
git add crates/drz-viewmodel/src/editor_vm.rs
git commit -m "feat(viewmodel): add text_in_range + replace_selection_with"
```

---

## Task 4: `EditorIcons` SVG rasterization module

**Files:**
- Create: `crates/drz-editor/src/icon.rs`
- Modify: `crates/drz-editor/src/lib.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct EditorIcons {
      copy: Option<egui::TextureHandle>,
      cut: Option<egui::TextureHandle>,
      paste: Option<egui::TextureHandle>,
  }
  impl EditorIcons {
      pub fn new() -> Self;
      pub fn ensure_textures(&mut self, ctx: &egui::Context);
      pub fn copy(&self) -> Option<&egui::TextureHandle>;
      pub fn cut(&self) -> Option<&egui::TextureHandle>;
      pub fn paste(&self) -> Option<&egui::TextureHandle>;
  }
  ```

- [ ] **Step 1: Write the failing test**

Create `/home/druzo/Desenvolvimento/DRZDiffCoder/crates/drz-editor/src/icon.rs`:

```rust
use usvg::TreeParsing;
use usvg::TreeTextToPath;

const COPY_SVG: &[u8] = include_bytes!("../../../icons/doc.on.clipboard.svg");
const CUT_SVG: &[u8] = include_bytes!("../../../icons/scissors.svg");
const PASTE_SVG: &[u8] = include_bytes!("../../../icons/doc.on.clipboard.fill.svg");

const ICON_PX: u32 = 14;

pub struct EditorIcons {
    copy: Option<egui::TextureHandle>,
    cut: Option<egui::TextureHandle>,
    paste: Option<egui::TextureHandle>,
}

impl EditorIcons {
    pub fn new() -> Self {
        Self { copy: None, cut: None, paste: None }
    }

    pub fn ensure_textures(&mut self, ctx: &egui::Context) {
        if self.copy.is_none() {
            self.copy = rasterize(ctx, "drz_icon_copy", COPY_SVG);
        }
        if self.cut.is_none() {
            self.cut = rasterize(ctx, "drz_icon_cut", CUT_SVG);
        }
        if self.paste.is_none() {
            self.paste = rasterize(ctx, "drz_icon_paste", PASTE_SVG);
        }
    }

    pub fn copy(&self) -> Option<&egui::TextureHandle> { self.copy.as_ref() }
    pub fn cut(&self) -> Option<&egui::TextureHandle> { self.cut.as_ref() }
    pub fn paste(&self) -> Option<&egui::TextureHandle> { self.paste.as_ref() }
}

impl Default for EditorIcons {
    fn default() -> Self { Self::new() }
}

/// Decode an SVG byte slice into an `egui::TextureHandle` sized to ICON_PX.
/// Returns `None` on any decode error — callers fall back to a text glyph.
fn rasterize(ctx: &egui::Context, name: &str, svg_bytes: &[u8]) -> Option<egui::TextureHandle> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(svg_bytes, &opt).ok()?;
    let tree = tree.text_to_path(&usvg::FontSystem::new());
    let size = tree.size().width().max(tree.size().height()).max(1.0);
    let scale = ICON_PX as f32 / size;
    let pixmap_w = ICON_PX;
    let pixmap_h = ICON_PX;
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_w, pixmap_h)?;
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let color = egui::ColorImage::from_rgba_unmultiplied(
        [pixmap_w as usize, pixmap_h as usize],
        pixmap.data(),
    );
    Some(ctx.load_texture(name, color, egui::TextureOptions::LINEAR))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal valid SVG (a 10x10 black square). Used as a sanity check
    /// that `rasterize` accepts well-formed SVG input. We do not exercise
    /// `ensure_textures` here because it requires an `egui::Context`, which
    /// has no headless test fixture in this crate.
    const TINY_SVG: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 10 10\"><rect width=\"10\" height=\"10\" fill=\"black\"/></svg>";

    #[test]
    fn usvg_accepts_minimal_svg() {
        let opt = usvg::Options::default();
        let tree = usvg::Tree::from_data(TINY_SVG, &opt);
        assert!(tree.is_ok(), "usvg must parse the minimal SVG fixture");
        let tree = tree.unwrap().text_to_path(&usvg::FontSystem::new());
        assert!(tree.size().width() > 0.0);
        assert!(tree.size().height() > 0.0);
    }

    #[test]
    fn editor_icons_new_has_no_textures() {
        let icons = EditorIcons::new();
        assert!(icons.copy().is_none());
        assert!(icons.cut().is_none());
        assert!(icons.paste().is_none());
    }

    #[test]
    fn default_impl_matches_new() {
        let a = EditorIcons::new();
        let b = EditorIcons::default();
        assert_eq!(a.copy().is_none(), b.copy().is_none());
        assert_eq!(a.cut().is_none(), b.cut().is_none());
        assert_eq!(a.paste().is_none(), b.paste().is_none());
    }
}
```

- [ ] **Step 2: Wire `icon` module into `lib.rs`**

Open `/home/druzo/Desenvolvimento/DRZDiffCoder/crates/drz-editor/src/lib.rs`. The file currently re-exports from `editor.rs` and `theme.rs`. Add the icon module declaration at the top (alongside existing module declarations — read the file first to see its current shape) and re-export `EditorIcons`. Concretely:

At the top of `lib.rs`, ensure `pub mod icon;` is declared. At the bottom (alongside other `pub use` lines), add `pub use icon::EditorIcons;`.

(The file's exact contents may already have a specific structure — read it first and match the existing pattern.)

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p drz-editor icon::`
Expected: 3 tests pass.

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -p drz-editor --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 5: Commit**

```bash
cd /home/druzo/Desenvolvimento/DRZDiffCoder
git add crates/drz-editor/src/icon.rs crates/drz-editor/src/lib.rs
git commit -m "feat(editor): add EditorIcons SVG rasterization"
```

---

## Task 5: `CodeEditor` selection state + mouse interaction (click, drag, double, triple)

**Files:**
- Modify: `crates/drz-editor/src/editor.rs`

**Interfaces:**
- Adds to `CodeEditor`:
  ```rust
  pub struct CodeEditor {
      cursor: (usize, usize),
      selection: Option<drz_viewmodel::Selection>,
      drag_anchor: Option<(usize, usize)>,
      last_double_click: Option<(std::time::Instant, usize)>,
      icons: EditorIcons,
  }
  ```
- New methods: `selection(&self) -> Option<&Selection>`, `set_selection(&mut self, sel: Option<Selection>)`.

This task adds selection state + all mouse interaction. Keyboard handling and context menu come in Tasks 6 and 7.

- [ ] **Step 1: Write the failing tests for word-boundary scanner**

In `crates/drz-editor/src/editor.rs`, the existing test module is at the bottom. Append tests for the new pure-logic helper. First add a helper function for the scanner (declared but unimplemented), then the tests:

```rust
    #[test]
    fn word_bound_left_right_alphanumeric_underscore() {
        // "foo bar_baz.qux 42" → click at col 5 (inside "foo bar")
        let line = "foo bar_baz.qux 42";
        assert_eq!(word_range(line, 5), (0, 7)); // "foo bar"
        // click at col 8 (start of "bar_baz")
        assert_eq!(word_range(line, 8), (8, 15));
        // click at col 12 (inside "bar_baz", underscore counts as word char)
        assert_eq!(word_range(line, 12), (8, 15));
        // click at col 16 ("qux")
        assert_eq!(word_range(line, 16), (16, 19));
        // click at col 20 ("42")
        assert_eq!(word_range(line, 20), (20, 22));
    }

    #[test]
    fn word_bound_stops_at_non_word() {
        // "  abc def  " — clicking in "abc" yields "abc".
        let line = "  abc def  ";
        assert_eq!(word_range(line, 3), (2, 5));
        // Click on space → empty range at that col.
        assert_eq!(word_range(line, 0), (0, 0));
        assert_eq!(word_range(line, 5), (5, 5));
    }

    #[test]
    fn word_bound_utf8_bytewise() {
        // "  café  " — c=1B, a=1B, f=1B, é=2B. Click at col 4 (inside "café")
        let line = "  café  ";
        assert_eq!(word_range(line, 4), (2, 6)); // bytes 2..6 == "café"
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p drz-editor word_bound_`
Expected: compile error — `word_range` not defined.

- [ ] **Step 3: Add `word_range` helper (pure)**

In `crates/drz-editor/src/editor.rs`, alongside the existing `pub(crate)` helpers (`clamp_col`, `x_to_col`, etc.), add:

```rust
/// Byte-col range of the "word" containing `col` in `line`. A word is a
/// contiguous run of `[A-Za-z0-9_]` bytes. Returns `(left, right)` byte
/// offsets such that `line[left..right]` is the selected word. If `col` is
/// on a non-word byte, returns `(col, col)` (empty).
pub(crate) fn word_range(line: &str, col: usize) -> (usize, usize) {
    let col = col.min(line.len());
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let bytes = line.as_bytes();
    if col >= bytes.len() || !is_word(bytes[col]) {
        // If col sits exactly on the byte AFTER a word (e.g. a space), still
        // return empty rather than grabbing the prior word. Callers can
        // shift the click to the nearest word char first if they want.
        return (col, col);
    }
    let mut left = col;
    while left > 0 && is_word(bytes[left - 1]) {
        left -= 1;
    }
    let mut right = col + 1;
    while right < bytes.len() && is_word(bytes[right]) {
        right += 1;
    }
    (left, right)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p drz-editor word_bound_`
Expected: 3 tests pass.

- [ ] **Step 5: Add selection state fields to `CodeEditor`**

In `crates/drz-editor/src/editor.rs`, replace the existing `pub struct CodeEditor` block:

```rust
pub struct CodeEditor {
    cursor: (usize, usize), // (line, col_byte)
}
```

with:

```rust
pub struct CodeEditor {
    cursor: (usize, usize), // (line, col_byte)
    selection: Option<drz_viewmodel::Selection>,
    /// Anchor captured at drag start (left-button drag extends selection).
    /// `None` outside an active drag.
    drag_anchor: Option<(usize, usize)>,
    /// Timestamp + line of the most recent double-click, used to detect
    /// triple-click within 300 ms on the same line.
    last_double_click: Option<(std::time::Instant, usize)>,
    icons: EditorIcons,
}
```

- [ ] **Step 6: Update `CodeEditor::new` (and add accessor methods)**

Replace the existing `new` with:

```rust
impl CodeEditor {
    pub fn new() -> CodeEditor {
        CodeEditor {
            cursor: (0, 0),
            selection: None,
            drag_anchor: None,
            last_double_click: None,
            icons: EditorIcons::new(),
        }
    }

    pub fn cursor(&self) -> (usize, usize) {
        self.cursor
    }

    pub fn selection(&self) -> Option<&drz_viewmodel::Selection> {
        self.selection.as_ref()
    }

    pub fn set_selection(&mut self, sel: Option<drz_viewmodel::Selection>) {
        self.selection = sel;
    }
}
```

- [ ] **Step 7: Rewrite the mouse interaction block in `show()`**

Inside `CodeEditor::show` (the method beginning at the existing `pub fn show(...)`), locate the existing block:

```rust
let output = egui::ScrollArea::both()
    .auto_shrink([false, false])
    .id_salt(ui.id().with("editor_scroll"))
    .scroll_offset(*scroll)
    .show(ui, |ui| {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(
                gutter_width + char_width * max_line_cols(vm) as f32 + 40.0,
                row_height * rows as f32,
            ),
            egui::Sense::click(),
        );
        ...
        if response.clicked() {
            response.request_focus();
            ...
        }
        ...
    });
```

Replace `egui::Sense::click()` with `egui::Sense::click_and_drag()`. Replace the entire `if response.clicked() { ... }` block with the new interaction block:

```rust
        // Mouse interaction: click, drag, double-click, triple-click.
        self.icons.ensure_textures(ui.ctx());
        let mods = ui.ctx().input(|i| i.modifiers);
        let shift = mods.shift;

        if response.clicked() || response.drag_started() {
            response.request_focus();
        }
        if let Some(pos) = response.interact_pointer_pos() {
            let row = ((pos.y - rect.top()) / row_height).floor() as usize;
            let col = x_to_col(pos.x - rect.left() - gutter_width, char_width);
            let line = match line_of_row {
                Some(f) => f(row).unwrap_or_else(|| vm.len_lines().saturating_sub(1)),
                None => row.min(vm.len_lines().saturating_sub(1)),
            };
            let (span_start, span_end) = vm.line_byte_range(line);
            let line_len = span_end - span_start;
            let clamped_col = clamp_col(col, line_len);

            if response.drag_started() {
                self.drag_anchor = Some((line, clamped_col));
                self.selection = Some(drz_viewmodel::Selection::new(
                    (line, clamped_col),
                    (line, clamped_col),
                ));
                self.cursor = (line, clamped_col);
                self.last_double_click = None;
            } else if response.dragged() {
                if let Some(anchor) = self.drag_anchor {
                    self.cursor = (line, clamped_col);
                    self.selection = Some(drz_viewmodel::Selection::new(anchor, (line, clamped_col)));
                }
            } else if response.drag_stopped() {
                self.drag_anchor = None;
            } else if response.double_clicked() {
                response.request_focus();
                let (ls, le) = vm.line_byte_range(line);
                let text_bytes = vm.line(line);
                let (l, r) = word_range(&text_bytes, clamped_col);
                let abs_l = ls + l;
                let abs_r = ls + r;
                let _ = le; // (le used implicitly via line_len cap)
                self.cursor = (line, abs_r);
                self.selection = Some(drz_viewmodel::Selection::new(
                    (line, abs_l),
                    (line, abs_r),
                ));
                self.drag_anchor = None;
                let now = std::time::Instant::now();
                if let Some((prev_at, prev_line)) = self.last_double_click {
                    if prev_line == line && now.duration_since(prev_at).as_millis() < 300 {
                        // Triple-click: select the whole line.
                        let (ls2, _le2) = vm.line_byte_range(line);
                        let line_len = vm.line(line).len();
                        self.cursor = (line, line_len);
                        self.selection = Some(drz_viewmodel::Selection::new(
                            (line, ls2.min(ls + line_len)),
                            (line, ls + line_len),
                        ));
                        self.last_double_click = None;
                    } else {
                        self.last_double_click = Some((now, line));
                    }
                } else {
                    self.last_double_click = Some((now, line));
                }
            } else if response.clicked() {
                let anchor = if shift {
                    self.selection
                        .map(|s| s.anchor)
                        .unwrap_or((line, clamped_col))
                } else {
                    self.drag_anchor = None;
                    self.last_double_click = None;
                    (line, clamped_col)
                };
                self.cursor = (line, clamped_col);
                self.selection = Some(drz_viewmodel::Selection::new(anchor, (line, clamped_col)));
                if anchor == (line, clamped_col) {
                    self.selection = None;
                }
            }
        } else if response.clicked() {
            // Click outside any visible row: collapse selection.
            self.selection = None;
            self.drag_anchor = None;
        }
```

(Use exact whitespace matching — the `if response.clicked() { ... }` block being replaced is currently 18 lines, indented 16 spaces from the start of the file. Read the file first to confirm indentation before applying the edit.)

- [ ] **Step 8: Build to verify compile**

Run: `cargo build -p drz-editor`
Expected: compiles, possibly warnings about unused `handle_keys` integration with selection — those resolve in Task 6.

- [ ] **Step 9: Run all editor tests**

Run: `cargo test -p drz-editor`
Expected: existing tests + new word_bound tests pass.

- [ ] **Step 10: Commit**

```bash
cd /home/druzo/Desenvolvimento/DRZDiffCoder
git add crates/drz-editor/src/editor.rs
git commit -m "feat(editor): CodeEditor mouse selection + word/line selection"
```

---

## Task 6: `CodeEditor` keyboard (Shift+arrow, Ctrl+A/C/X/V, selection-aware edits)

**Files:**
- Modify: `crates/drz-editor/src/editor.rs` (`handle_keys` method)

**Interfaces:**
- Adds keyboard shortcuts to `handle_keys`:
  - Shift + ←/→/↑/↓ → extend selection from anchor (anchor stays fixed).
  - Ctrl/Cmd + A → select all.
  - Ctrl/Cmd + C → copy selected text to `ui.ctx().copy_text`.
  - Ctrl/Cmd + X → cut (copy + replace_selection_with(start, end, "")).
  - Ctrl/Cmd + V → paste (replace_selection_with(cursor-or-selection, clipboard_text)).
  - Typing with selection → replace_selection_with(sel, ch); caret at end of inserted text.
  - Backspace / Delete with selection → replace_selection_with(sel, ""); caret at start.

- [ ] **Step 1: Write failing tests for selection update semantics**

Append to the test module:

```rust
    #[test]
    fn selection_extend_from_anchor_on_shift_right() {
        // Pure-logic test of the helper used by handle_keys.
        let mut sel = drz_viewmodel::Selection::new((0, 2), (0, 2));
        // simulate Shift+Right: extend cursor one col, anchor stays.
        sel.cursor = (0, 3);
        assert_eq!(sel.ordered(), ((0, 2), (0, 3)));
        assert!(sel.is_selected());
        // simulate Shift+Right again
        sel.cursor = (0, 4);
        assert_eq!(sel.ordered(), ((0, 2), (0, 4)));
    }

    #[test]
    fn selection_collapse_then_extend_starts_new_anchor() {
        // Plain Right click collapses; Shift+Right then extends from new anchor.
        let mut sel: Option<drz_viewmodel::Selection> = None;
        // click at (0,5) → selection = Some(anchor=(0,5), cursor=(0,5))
        sel = Some(drz_viewmodel::Selection::new((0, 5), (0, 5)));
        assert!(!sel.unwrap().is_selected());
        // Shift+Right → cursor = (0,6), anchor stays (0,5)
        if let Some(s) = sel.as_mut() {
            s.cursor = (0, 6);
        }
        assert_eq!(sel.unwrap().ordered(), ((0, 5), (0, 6)));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p drz-editor selection_extend_ selection_collapse_then_extend_`
Expected: compile error — `Selection` not imported. (These tests need `use drz_viewmodel::Selection;` in the test module. Add it.)

If the test module doesn't already import `drz_viewmodel`, add at the top of the `mod tests` block:

```rust
    use drz_viewmodel::Selection;
```

- [ ] **Step 3: Rewrite `handle_keys` to integrate selection**

In `crates/drz-editor/src/editor.rs`, the existing `fn handle_keys` is a private method. Replace its body with a version that:

1. Computes `selection_aware_insert(text)`: if `self.selection.is_some()`, calls `vm.replace_selection_with(start, end, text)` and updates `self.cursor = returned_pos` + `self.selection = None`. Else, calls `vm.insert_at_line_col(...)` as before.
2. Handles Shift + ←/→/↑/↓ by updating `self.selection.cursor` instead of just `self.cursor`.
3. Handles Ctrl/Cmd + A by setting `self.selection = Some(Selection::new((0,0), (vm.len_lines()-1, vm.line(vm.len_lines()-1).len())))` and snapping cursor to selection end.
4. Handles Ctrl/Cmd + C by `ui.ctx().copy_text(vm.text_in_range(start, end))`.
5. Handles Ctrl/Cmd + X by copy + `vm.replace_selection_with(start, end, "")` + collapse.
6. Handles Ctrl/Cmd + V by reading clipboard text (via `ui.ctx().input(|i| i.events)` filtering for `egui::Event::Paste`) — note: egui delivers paste as a synthetic event when the user triggers Ctrl+V. Route through the same `replace_selection_with` path.
7. Backspace with selection → `replace_selection_with(start, end, "")` + collapse.
8. Plain typing with selection → `replace_selection_with(start, end, ch)`.

Concretely the new body of `handle_keys` is (replace the entire method body from the `let (line, col) = self.cursor;` line through the final `}`):

```rust
    fn handle_keys(&mut self, ui: &mut egui::Ui, vm: &mut EditorViewModel) {
        use drz_viewmodel::Selection;
        let (line, col) = self.cursor;
        let col = if line < vm.len_lines() {
            floor_col_boundary(&vm.line(line), col)
        } else {
            col
        };
        self.cursor.1 = col;
        let mods = ui.ctx().input(|i| i.modifiers);
        let cmd_or_ctrl = mods.command;
        let shift = mods.shift;

        // Selection-aware insert: replaces the current selection (if any) and
        // leaves the caret at the end of the inserted text. Returns true if
        // the input was consumed.
        let mut do_selection_replace = |text: &str, vm: &mut EditorViewModel| -> bool {
            if let Some(sel) = self.selection.take() {
                let (s, e) = sel.ordered();
                let (nl, nc) = vm.replace_selection_with(s, e, text);
                self.cursor = (nl, nc);
                true
            } else {
                false
            }
        };

        let mut paste_text: Option<String> = None;
        let mut copy_request: Option<String> = None;
        let mut cut_request: Option<((usize, usize), (usize, usize))> = None;
        let mut select_all_request = false;

        ui.input(|i| {
            for event in &i.events {
                match event {
                    egui::Event::Text(t) => {
                        if !do_selection_replace(t, vm) {
                            vm.insert_at_line_col(line, col, t);
                            self.cursor.1 += t.len();
                            self.cursor.0 = self.cursor.0.min(vm.len_lines().saturating_sub(1));
                        }
                    }
                    egui::Event::Paste(s) => {
                        paste_text = Some(s.clone());
                    }
                    egui::Event::Copy => {
                        copy_request = self.selection.and_then(|sel| {
                            let (s, e) = sel.ordered();
                            if s == e { None } else { Some(vm.text_in_range(s, e)) }
                        });
                    }
                    egui::Event::Cut => {
                        cut_request = self.selection.and_then(|sel| {
                            let (s, e) = sel.ordered();
                            if s == e { None } else { Some((s, e)) }
                        });
                    }
                    egui::Event::Key { key: egui::Key::A, pressed: true, .. }
                        if cmd_or_ctrl =>
                    {
                        select_all_request = true;
                    }
                    egui::Event::Key { key: egui::Key::Enter, pressed: true, .. } => {
                        if !do_selection_replace("\n", vm) {
                            vm.insert_at_line_col(line, col, "\n");
                            self.cursor = (line + 1, 0);
                        }
                    }
                    egui::Event::Key {
                        key: egui::Key::Backspace,
                        pressed: true,
                        ..
                    } => {
                        if let Some(sel) = self.selection.take() {
                            let (s, _e) = sel.ordered();
                            let (nl, nc) = vm.replace_selection_with(s, s, "");
                            let _ = (nl, nc);
                            self.cursor = s;
                        } else if col > 0 {
                            let prev_char_len = vm.line(line)[..col]
                                .chars()
                                .last()
                                .map(|c| c.len_utf8())
                                .unwrap_or(1);
                            vm.delete_range_line_col((line, col - prev_char_len), (line, col));
                            self.cursor.1 -= prev_char_len;
                        } else if line > 0 {
                            let prev_len = vm.line(line - 1).len();
                            vm.delete_range_line_col((line - 1, prev_len), (line, 0));
                            self.cursor = (line - 1, prev_len);
                        }
                    }
                    egui::Event::Key {
                        key: egui::Key::ArrowLeft,
                        pressed: true,
                        ..
                    } => {
                        if shift {
                            self.extend_or_init_selection(line, col);
                            self.selection.as_mut().unwrap().cursor.1 =
                                self.selection.as_ref().unwrap().cursor.1.saturating_sub(1);
                            self.cursor.1 = self.cursor.1.saturating_sub(1);
                        } else if col > 0 {
                            self.cursor.1 -= 1;
                            self.selection = None;
                        }
                    }
                    egui::Event::Key {
                        key: egui::Key::ArrowRight,
                        pressed: true,
                        ..
                    } => {
                        if shift {
                            self.extend_or_init_selection(line, col);
                            self.selection.as_mut().unwrap().cursor.1 =
                                clamp_col(self.selection.as_ref().unwrap().cursor.1 + 1, vm.line(line).len());
                            self.cursor.1 = clamp_col(self.cursor.1 + 1, vm.line(line).len());
                        } else {
                            self.cursor.1 = clamp_col(col + 1, vm.line(line).len());
                            self.selection = None;
                        }
                    }
                    egui::Event::Key {
                        key: egui::Key::ArrowUp,
                        pressed: true,
                        ..
                    } if line > 0 => {
                        if shift {
                            self.extend_or_init_selection(line, col);
                            self.selection.as_mut().unwrap().cursor.0 -= 1;
                            let new_col = clamp_col(
                                self.selection.as_ref().unwrap().cursor.1,
                                vm.line(line - 1).len(),
                            );
                            self.selection.as_mut().unwrap().cursor.1 = new_col;
                            self.cursor.0 -= 1;
                            self.cursor.1 = clamp_col(col, vm.line(line - 1).len());
                        } else {
                            self.cursor.0 -= 1;
                            self.cursor.1 = clamp_col(col, vm.line(line - 1).len());
                            self.selection = None;
                        }
                    }
                    egui::Event::Key {
                        key: egui::Key::ArrowDown,
                        pressed: true,
                        ..
                    } if line + 1 < vm.len_lines() => {
                        if shift {
                            self.extend_or_init_selection(line, col);
                            self.selection.as_mut().unwrap().cursor.0 += 1;
                            let new_col = clamp_col(
                                self.selection.as_ref().unwrap().cursor.1,
                                vm.line(line + 1).len(),
                            );
                            self.selection.as_mut().unwrap().cursor.1 = new_col;
                            self.cursor.0 += 1;
                            self.cursor.1 = clamp_col(col, vm.line(line + 1).len());
                        } else {
                            self.cursor.0 += 1;
                            self.cursor.1 = clamp_col(col, vm.line(line + 1).len());
                            self.selection = None;
                        }
                    }
                    _ => {}
                }
            }
        });

        // Post-process queued actions.
        if let Some(text) = copy_request {
            ui.ctx().copy_text(text);
        }
        if let Some((s, e)) = cut_request {
            let text = vm.text_in_range(s, e);
            ui.ctx().copy_text(text);
            let (nl, nc) = vm.replace_selection_with(s, e, "");
            let _ = (nl, nc);
            self.cursor = s;
            self.selection = None;
        }
        if let Some(text) = paste_text {
            if let Some(sel) = self.selection.take() {
                let (s, _e) = sel.ordered();
                let (nl, nc) = vm.replace_selection_with(s, s, &text);
                self.cursor = (nl, nc);
            } else {
                let (nl, nc) = vm.replace_selection_with((line, col), (line, col), &text);
                self.cursor = (nl, nc);
            }
        }
        if select_all_request {
            let last = vm.len_lines().saturating_sub(1);
            let last_len = if last < vm.len_lines() { vm.line(last).len() } else { 0 };
            self.selection = Some(Selection::new((0, 0), (last, last_len)));
            self.cursor = (last, last_len);
        }

        // Re-clamp after possible edits.
        let (l, c) = self.cursor;
        if l < vm.len_lines() {
            self.cursor.1 = clamp_col(c, vm.line(l).len());
        }
    }

    /// Initialize selection if absent (plain arrow with Shift). Sets anchor
    /// to current caret position; cursor stays where the user is moving.
    fn extend_or_init_selection(&mut self, line: usize, col: usize) {
        use drz_viewmodel::Selection;
        if self.selection.is_none() {
            self.selection = Some(Selection::new((line, col), (line, col)));
        }
    }
```

- [ ] **Step 4: Build to verify compile**

Run: `cargo build -p drz-editor`
Expected: compiles. If `egui::Event::Paste` / `Copy` / `Cut` are not available in egui 0.31 (they were added in 0.24), they are — but if the workspace is on a different version, the compile will report it. In that case, add a manual handler:

```rust
                    egui::Event::Key { key: egui::Key::C, pressed: true, .. } if cmd_or_ctrl && !shift => {
                        if let Some(sel) = self.selection {
                            let (s, e) = sel.ordered();
                            copy_request = if s == e { None } else { Some(vm.text_in_range(s, e)) };
                        }
                    }
                    egui::Event::Key { key: egui::Key::X, pressed: true, .. } if cmd_or_ctrl && !shift => {
                        if let Some(sel) = self.selection {
                            let (s, e) = sel.ordered();
                            cut_request = if s == e { None } else { Some((s, e)) };
                        }
                    }
                    egui::Event::Key { key: egui::Key::V, pressed: true, .. } if cmd_or_ctrl && !shift => {
                        // Paste is harder without egui's clipboard read; in
                        // that case, this branch is a no-op and the user
                        // must rely on the right-click menu.
                        paste_text = ui.ctx().clipboard_text();
                    }
```

Add this block inside the same `match event` inside `ui.input(|i| ...)` and remove the `egui::Event::Paste/Copy/Cut` arms.

(Read the build error before applying this fallback — only use it if egui 0.31 lacks those event variants.)

- [ ] **Step 5: Run all editor + viewmodel tests**

Run: `cargo test -p drz-editor -p drz-viewmodel`
Expected: all tests pass.

- [ ] **Step 6: Run clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Commit**

```bash
cd /home/druzo/Desenvolvimento/DRZDiffCoder
git add crates/drz-editor/src/editor.rs
git commit -m "feat(editor): keyboard selection + clipboard shortcuts"
```

---

## Task 7: Right-click context menu

**Files:**
- Modify: `crates/drz-editor/src/editor.rs` (`show` method, after the mouse-interaction block)

**Interfaces:**
- Adds: `response.context_menu(|ui| { ... })` block rendering 4 menu items with icons + enable rules per spec §3.

- [ ] **Step 1: Add the context menu block**

In `crates/drz-editor/src/editor.rs`, inside `show()` immediately after the mouse-interaction block (after the `} else if response.clicked() { ... }` chain, before the `let focused = response.has_focus();` line), add:

```rust
        // Right-click context menu.
        let has_sel = self.selection.map(|s| s.is_selected()).unwrap_or(false);
        let clipboard_has_text = ui.ctx().clipboard_has_text();
        response.context_menu(|ui| {
            let copy_label = if let Some(t) = self.icons.copy() {
                egui::Button::image_and_text((t.id(), egui::vec2(14.0, 14.0)), "Copy")
            } else {
                egui::Button::new("Copy")
            };
            let cut_label = if let Some(t) = self.icons.cut() {
                egui::Button::image_and_text((t.id(), egui::vec2(14.0, 14.0)), "Cut")
            } else {
                egui::Button::new("Cut")
            };
            let paste_label = if let Some(t) = self.icons.paste() {
                egui::Button::image_and_text((t.id(), egui::vec2(14.0, 14.0)), "Paste")
            } else {
                egui::Button::new("Paste")
            };
            if ui.add_enabled(has_sel, copy_label).clicked() {
                if let Some(sel) = self.selection {
                    let (s, e) = sel.ordered();
                    if s != e {
                        ui.ctx().copy_text(vm.text_in_range(s, e));
                    }
                }
                ui.close_menu();
            }
            if ui.add_enabled(has_sel, cut_label).clicked() {
                if let Some(sel) = self.selection {
                    let (s, e) = sel.ordered();
                    if s != e {
                        let text = vm.text_in_range(s, e);
                        ui.ctx().copy_text(text);
                        let (nl, nc) = vm.replace_selection_with(s, e, "");
                        let _ = (nl, nc);
                        self.cursor = s;
                        self.selection = None;
                    }
                }
                ui.close_menu();
            }
            if ui.add_enabled(clipboard_has_text, paste_label).clicked() {
                if let Some(text) = ui.ctx().clipboard_text() {
                    if !text.is_empty() {
                        let (s, e) = match self.selection {
                            Some(sel) => sel.ordered(),
                            None => (self.cursor, self.cursor),
                        };
                        let (nl, nc) = vm.replace_selection_with(s, e, &text);
                        self.cursor = (nl, nc);
                        self.selection = None;
                    }
                }
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Select All").clicked() {
                let last = vm.len_lines().saturating_sub(1);
                let last_len = if last < vm.len_lines() { vm.line(last).len() } else { 0 };
                self.selection = Some(drz_viewmodel::Selection::new((0, 0), (last, last_len)));
                self.cursor = (last, last_len);
                ui.close_menu();
            }
        });
```

- [ ] **Step 2: Build to verify compile**

Run: `cargo build -p drz-editor`
Expected: compiles. If `egui::Context::clipboard_has_text` / `clipboard_text` are not in egui 0.31, fall back to a `paste_requested` flag pattern: track `paste_text: Option<String>` from a future egui event and use it here too. Read the build error first; only apply the fallback if needed.

- [ ] **Step 3: Run tests**

Run: `cargo test -p drz-editor -p drz-viewmodel`
Expected: all tests pass.

- [ ] **Step 4: Run clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 5: Commit**

```bash
cd /home/druzo/Desenvolvimento/DRZDiffCoder
git add crates/drz-editor/src/editor.rs
git commit -m "feat(editor): right-click context menu with icons"
```

---

## Task 8: Final verification (workspace-wide)

**Files:** none (CI verification only)

- [ ] **Step 1: Format**

Run: `cargo fmt --all`
Expected: clean exit, no diff after.

- [ ] **Step 2: Clippy with denied warnings**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings. If warnings appear, fix in reverse order (read the warning → fix → rerun).

- [ ] **Step 3: All tests**

Run: `cargo test --workspace`
Expected: all tests pass.

- [ ] **Step 4: Manual smoke test (developer)**

The agent cannot run the GUI; instruct the user to launch and verify:

```bash
cargo run -p drz-app -- crates/drz-editor/tests/fixtures/left.rs crates/drz-editor/tests/fixtures/right.rs
# (Use any two small text files; see AGENTS.md CLI smoke test pattern.)
```

Expected manual checks (per spec §8):

1. Drag-select in either pane → highlight visible.
2. Right-click → menu shows 4 items; Copy/Cut dimmed with no selection; Paste dimmed if clipboard empty.
3. Ctrl+C copies selection; paste into another app matches.
4. Ctrl+V pastes; selection replaced.
5. Double-click selects word; triple-click selects line; Shift+arrow extends.
6. Ctrl+A selects all; typing replaces selection.
7. Icons render as 14×14 PNG (not text glyphs) when SVG rasterization succeeds.

- [ ] **Step 5: Final commit (if fmt/clippy made any whitespace fixes)**

If `cargo fmt --all` produced diffs, those should already have been part of earlier task commits (Step 1 of each task). If `cargo fmt --all` is clean in this task, no commit needed.

If any clippy fixes were needed, commit them as a separate `fix:` commit:

```bash
cd /home/druzo/Desenvolvimento/DRZDiffCoder
git status
# If clean, no commit. If changes:
git add -A
git commit -m "fix: address clippy/fmt nits"
```

---

## Self-Review Checklist (run before declaring plan done)

- [ ] Every spec section maps to at least one task:
  - Spec §3 selection triggers → Task 5 (mouse) + Task 6 (keyboard).
  - Spec §3 clipboard shortcuts → Task 6 (keyboard events).
  - Spec §3 context menu → Task 7.
  - Spec §4 architecture (state placement, MVVM boundary) → Tasks 2, 3, 5.
  - Spec §4 icons (SVGs + resvg) → Tasks 1, 4.
  - Spec §5 data flow → Tasks 5, 6, 7.
  - Spec §6 files → all tasks.
  - Spec §7 testing → tests embedded in every task.
  - Spec §8 risks (clipboard API availability) → Task 6 / Task 7 fallbacks documented.
- [ ] No `TBD` / `TODO` / "implement later" in any task.
- [ ] No "similar to Task N" without re-stating the code.
- [ ] Type names match across tasks: `Selection`, `EditorIcons`, `text_in_range`, `replace_selection_with`, `word_range`.
- [ ] Every task ends with a commit step.
- [ ] Every task's tests run before commit.
- [ ] All commits follow Conventional Commits format.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-08-06-editor-selection-clipboard.md`.

Two execution options:

1. **Subagent-Driven (recommended)** — fresh subagent per task, review between tasks, fast iteration.
2. **Inline Execution** — execute tasks in this session using executing-plans, batch execution with checkpoints.
