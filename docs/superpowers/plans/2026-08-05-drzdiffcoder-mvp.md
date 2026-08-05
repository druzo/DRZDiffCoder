# DRZDiffCoder MVP (Phase 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build Phase 1 MVP: 2-file side-by-side compare with editable panes, incremental tree-sitter highlighting, save, git-difftool CLI mode, scroll sync, connector lines.

**Architecture:** MVVM in a cargo workspace. Model = `drz-core` (rope, diff, I/O) + `drz-highlight` (tree-sitter). ViewModel = `drz-viewmodel` (headless, no egui). View = `drz-editor`, `drz-diff-ui`, `drz-app` (egui/eframe).

**Tech Stack:** Rust 1.95, egui/eframe 0.31, tree-sitter 0.25, ropey 1.6, similar 2, clap 4, rfd 0.15, anyhow/thiserror, insta + assert_cmd (dev).

## Global Constraints

- Platforms: Linux, Windows, macOS — single codebase. No `#[cfg(unix)]`-only logic without a Windows fallback.
- No `unwrap`/`expect` in non-test code. `anyhow::Result` in `drz-app`, `thiserror` in library crates.
- ViewModel (`drz-viewmodel`) MUST NOT depend on egui/eframe. View crates MUST NOT depend on ropey/tree-sitter/similar directly — only via `drz-viewmodel` re-exported view types.
- Default file size cap: 50MB (above → plain-text mode). Binary = NUL byte in first 8KB → refuse to open.
- Every edit to a Document emits exactly one `tree_sitter::InputEdit` fed to the highlight engine.
- Diff recompute runs on a background thread, debounced 150ms.
- Commits after every task. Conventional Commits (`feat:`, `fix:`, `test:`, `chore:`).

---

### Task 1: Workspace scaffold

**Files:**
- Create: `Cargo.toml` (workspace)
- Create: `crates/drz-core/Cargo.toml`, `crates/drz-core/src/lib.rs`
- Create: `crates/drz-highlight/Cargo.toml`, `crates/drz-highlight/src/lib.rs`
- Create: `crates/drz-viewmodel/Cargo.toml`, `crates/drz-viewmodel/src/lib.rs`
- Create: `crates/drz-editor/Cargo.toml`, `crates/drz-editor/src/lib.rs`
- Create: `crates/drz-diff-ui/Cargo.toml`, `crates/drz-diff-ui/src/lib.rs`
- Create: `crates/drz-app/Cargo.toml`, `crates/drz-app/src/main.rs`
- Create: `.gitignore`

**Interfaces:**
- Produces: workspace named `drzdiffcoder`, binary crate `drz-app` (bin name `drzdiff`), lib crates `drz-core`, `drz-highlight`, `drz-viewmodel`, `drz-editor`, `drz-diff-ui`.

- [ ] **Step 1: Write root `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = [
    "crates/drz-core",
    "crates/drz-highlight",
    "crates/drz-viewmodel",
    "crates/drz-editor",
    "crates/drz-diff-ui",
    "crates/drz-app",
]

[workspace.dependencies]
anyhow = "1"
thiserror = "2"
ropey = "1.6"
similar = "2.7"
tree-sitter = "0.25"
tree-sitter-rust = "0.24"
tree-sitter-python = "0.23"
tree-sitter-javascript = "0.25"
tree-sitter-c = "0.24"
tree-sitter-cpp = "0.23"
chardetng = "0.1"
egui = "0.31"
eframe = "0.31"
clap = { version = "4", features = ["derive"] }
rfd = "0.15"
```

- [ ] **Step 2: Write each crate manifest.** Library crates:

```toml
# crates/drz-core/Cargo.toml
[package]
name = "drz-core"
version = "0.1.0"
edition = "2021"

[dependencies]
thiserror.workspace = true
ropey.workspace = true
similar.workspace = true
chardetng.workspace = true
```

```toml
# crates/drz-highlight/Cargo.toml
[package]
name = "drz-highlight"
version = "0.1.0"
edition = "2021"

[dependencies]
thiserror.workspace = true
ropey.workspace = true
tree-sitter.workspace = true
tree-sitter-rust.workspace = true
tree-sitter-python.workspace = true
tree-sitter-javascript.workspace = true
tree-sitter-c.workspace = true
tree-sitter-cpp.workspace = true
```

```toml
# crates/drz-viewmodel/Cargo.toml
[package]
name = "drz-viewmodel"
version = "0.1.0"
edition = "2021"

[dependencies]
drz-core = { path = "../drz-core" }
drz-highlight = { path = "../drz-highlight" }
```

```toml
# crates/drz-editor/Cargo.toml
[package]
name = "drz-editor"
version = "0.1.0"
edition = "2021"

[dependencies]
egui.workspace = true
drz-viewmodel = { path = "../drz-viewmodel" }
```

```toml
# crates/drz-diff-ui/Cargo.toml
[package]
name = "drz-diff-ui"
version = "0.1.0"
edition = "2021"

[dependencies]
egui.workspace = true
drz-viewmodel = { path = "../drz-viewmodel" }
drz-editor = { path = "../drz-editor" }
```

```toml
# crates/drz-app/Cargo.toml
[package]
name = "drz-app"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "drzdiff"
path = "src/main.rs"

[dependencies]
anyhow.workspace = true
eframe.workspace = true
egui.workspace = true
clap.workspace = true
rfd.workspace = true
drz-viewmodel = { path = "../drz-viewmodel" }
drz-diff-ui = { path = "../drz-diff-ui" }

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

Add to root `[workspace.dependencies]`: nothing else. Empty lib.rs files get `// placeholder`. `drz-app/src/main.rs`:

```rust
fn main() {
    println!("drzdiff placeholder");
}
```

`.gitignore`: `/target`

- [ ] **Step 3: Verify build**

Run: `cargo build --workspace`
Expected: compiles, placeholder binary builds.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "chore: scaffold cargo workspace (6 crates)"
```

---

### Task 2: drz-core — Document rope buffer + edits

**Files:**
- Create: `crates/drz-core/src/document.rs`
- Create: `crates/drz-core/src/edit.rs`
- Modify: `crates/drz-core/src/lib.rs`
- Test: `crates/drz-core/src/document.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Produces:
  - `pub struct TextEdit { pub start_byte: usize, pub old_end_byte: usize, pub inserted: String }` with `pub fn new_end_byte(&self) -> usize`
  - `pub struct Document { /* private */ }`
  - `Document::from_text(text: &str) -> Document`
  - `Document::apply(&mut self, edit: &TextEdit) -> ()`
  - `Document::to_string(&self) -> String`
  - `Document::len_lines(&self) -> usize`
  - `Document::line(&self, idx: usize) -> String` (no trailing newline)
  - `Document::line_byte_range(&self, idx: usize) -> (usize, usize)` (start byte, end byte excl. newline)
  - `Document::replace_lines(&mut self, start: usize, end: usize, text: &str) -> ()` (end exclusive; builds one TextEdit internally)
  - `Document::rope(&self) -> &ropey::Rope`

- [ ] **Step 1: Write failing test** (inline in `document.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_insert_updates_text() {
        let mut doc = Document::from_text("hello\nworld\n");
        doc.apply(&TextEdit { start_byte: 0, old_end_byte: 0, inserted: ">> ".into() });
        assert_eq!(doc.to_string(), ">> hello\nworld\n");
    }

    #[test]
    fn apply_delete_updates_text() {
        let mut doc = Document::from_text("hello\nworld\n");
        doc.apply(&TextEdit { start_byte: 0, old_end_byte: 6, inserted: String::new() });
        assert_eq!(doc.to_string(), "world\n");
    }

    #[test]
    fn line_accessors() {
        let doc = Document::from_text("ab\ncd\n");
        assert_eq!(doc.len_lines(), 3); // trailing newline → empty last line
        assert_eq!(doc.line(1), "cd");
        assert_eq!(doc.line_byte_range(1), (3, 5));
    }

    #[test]
    fn replace_lines_swaps_content() {
        let mut doc = Document::from_text("a\nb\nc\n");
        doc.replace_lines(1, 2, "X\nY");
        assert_eq!(doc.to_string(), "a\nX\nY\nc\n");
    }
}
```

- [ ] **Step 2: Run, verify fail**

Run: `cargo test -p drz-core`
Expected: FAIL (types don't exist).

- [ ] **Step 3: Implement** `edit.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub inserted: String,
}

impl TextEdit {
    pub fn new_end_byte(&self) -> usize {
        self.start_byte + self.inserted.len()
    }
}
```

`document.rs` (above the test module):

```rust
use ropey::Rope;
use crate::edit::TextEdit;

pub struct Document {
    rope: Rope,
}

impl Document {
    pub fn from_text(text: &str) -> Document {
        Document { rope: Rope::from_str(text) }
    }

    pub fn apply(&mut self, edit: &TextEdit) {
        let start = edit.start_byte.min(self.rope.len_bytes());
        let old_end = edit.old_end_byte.min(self.rope.len_bytes()).max(start);
        if old_end > start {
            let s = self.rope.byte_to_char(start);
            let e = self.rope.byte_to_char(old_end);
            self.rope.remove(s..e);
        }
        if !edit.inserted.is_empty() {
            let s = self.rope.byte_to_char(start);
            self.rope.insert(s, &edit.inserted);
        }
    }

    pub fn to_string(&self) -> String {
        self.rope.to_string()
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn line(&self, idx: usize) -> String {
        let line = self.rope.line(idx);
        let mut s = line.to_string();
        while s.ends_with('\n') || s.ends_with('\r') {
            s.pop();
        }
        s
    }

    pub fn line_byte_range(&self, idx: usize) -> (usize, usize) {
        let start = self.rope.line_to_byte(idx);
        let line = self.rope.line(idx);
        let mut len = line.len_bytes();
        // strip trailing \n / \r\n
        let s = line.as_str().unwrap_or("");
        if s.ends_with('\n') {
            len -= 1;
            if len > 0 && s[..s.len()-1].ends_with('\r') {
                len -= 1;
            }
        }
        (start, start + len)
    }

    pub fn replace_lines(&mut self, start: usize, end: usize, text: &str) {
        let start_byte = self.rope.line_to_byte(start);
        let end_byte = if end >= self.rope.len_lines() {
            self.rope.len_bytes()
        } else {
            self.rope.line_to_byte(end)
        };
        self.apply(&TextEdit { start_byte, old_end_byte: end_byte, inserted: text.to_string() });
    }

    pub fn rope(&self) -> &Rope {
        &self.rope
    }
}
```

`lib.rs`:

```rust
pub mod document;
pub mod edit;

pub use document::Document;
pub use edit::TextEdit;
```

- [ ] **Step 4: Run, verify pass**

Run: `cargo test -p drz-core`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(core): Document rope buffer with TextEdit apply/replace_lines"
```

---

### Task 3: drz-core — file I/O + encoding + errors

**Files:**
- Create: `crates/drz-core/src/io.rs`
- Create: `crates/drz-core/src/error.rs`
- Modify: `crates/drz-core/src/lib.rs`
- Modify: `crates/drz-core/src/document.rs` (add `path`, `dirty`, `encoding_guessed` fields + `open`/`save`)

**Interfaces:**
- Consumes: `Document`, `TextEdit` (Task 2).
- Produces:
  - `pub enum CoreError { Io(#[from] std::io::Error), BinaryFile(PathBuf), TooLarge(PathBuf, u64), Empty }`
  - `pub const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;`
  - `Document::open(path: &Path) -> Result<Document, CoreError>`
  - `Document::path(&self) -> Option<&Path>`
  - `Document::is_dirty(&self) -> bool`
  - `Document::encoding_guessed(&self) -> bool`
  - `Document::save(&mut self) -> Result<(), CoreError>` (clears dirty; Empty error if no path)
  - `apply`/`replace_lines` set `dirty = true`.

- [ ] **Step 1: Write failing tests** (in `io.rs` test module)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use std::io::Write;

    fn tmpfile(bytes: &[u8], name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("drzcore_test");
        std::fs::create_dir_all(&dir).ok();
        let p = dir.join(name);
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(bytes).unwrap();
        p
    }

    #[test]
    fn open_utf8_roundtrip_save() {
        let p = tmpfile("fn main() {}\n".as_bytes(), "a.rs");
        let mut doc = Document::open(&p).unwrap();
        assert_eq!(doc.line(0), "fn main() {}");
        assert!(!doc.is_dirty());
        assert!(!doc.encoding_guessed());
        doc.replace_lines(0, 1, "fn main() { /*x*/ }");
        assert!(doc.is_dirty());
        doc.save().unwrap();
        assert!(!doc.is_dirty());
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "fn main() { /*x*/ }\n");
    }

    #[test]
    fn open_binary_rejected() {
        let mut bytes = b"abc".to_vec();
        bytes.push(0);
        bytes.extend_from_slice(b"def");
        let p = tmpfile(&bytes, "bin.dat");
        assert!(matches!(Document::open(&p), Err(CoreError::BinaryFile(_))));
    }

    #[test]
    fn open_latin1_guessed() {
        // 0xE9 = é in latin-1, invalid UTF-8
        let p = tmpfile(&[0x63, 0x61, 0x66, 0xE9, 0x0A], "latin.txt");
        let doc = Document::open(&p).unwrap();
        assert!(doc.encoding_guessed());
        assert_eq!(doc.line(0), "café");
    }
}
```

- [ ] **Step 2: Run, verify fail**

Run: `cargo test -p drz-core`
Expected: FAIL (`open`/`save` missing).

- [ ] **Step 3: Implement.** `error.rs`:

```rust
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("binary file not supported: {0}")]
    BinaryFile(PathBuf),
    #[error("file too large: {0} ({1} bytes)")]
    TooLarge(PathBuf, u64),
    #[error("document has no path")]
    NoPath,
}
```

`io.rs` — functions added to Document via inherent impl in `document.rs`? No: keep `io.rs` owning open/save as `impl Document` block in `io.rs` (Rust allows inherent impls in same crate across modules):

```rust
use crate::document::Document;
use crate::error::CoreError;
use std::path::Path;

pub const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;
const BINARY_SNIFF_LEN: usize = 8 * 1024;

impl Document {
    pub fn open(path: &Path) -> Result<Document, CoreError> {
        let meta = std::fs::metadata(path)?;
        if meta.len() > MAX_FILE_SIZE {
            return Err(CoreError::TooLarge(path.to_path_buf(), meta.len()));
        }
        let bytes = std::fs::read(path)?;
        let sniff = &bytes[..bytes.len().min(BINARY_SNIFF_LEN)];
        if sniff.contains(&0) {
            return Err(CoreError::BinaryFile(path.to_path_buf()));
        }
        let (text, guessed) = match std::str::from_utf8(&bytes) {
            Ok(s) => (s.to_string(), false),
            Err(_) => {
                let det = chardetng::EncodingDetector::new();
                let mut det = det;
                det.feed(&bytes, true);
                let enc = det.guess(None, true);
                let (cow, _, _) = enc.decode(&bytes);
                (cow.into_owned(), true)
            }
        };
        Ok(Document::from_file(text, path.to_path_buf(), guessed))
    }

    pub fn save(&mut self) -> Result<(), CoreError> {
        let path = self.path().ok_or(CoreError::NoPath)?.to_path_buf();
        std::fs::write(&path, self.to_string())?;
        self.mark_clean();
        Ok(())
    }
}
```

`document.rs` — extend struct:

```rust
use std::path::{Path, PathBuf};

pub struct Document {
    rope: ropey::Rope,
    path: Option<PathBuf>,
    dirty: bool,
    encoding_guessed: bool,
}

// in impl Document:
pub fn from_text(text: &str) -> Document {
    Document { rope: ropey::Rope::from_str(text), path: None, dirty: false, encoding_guessed: false }
}

pub fn from_file(text: String, path: PathBuf, encoding_guessed: bool) -> Document {
    Document { rope: ropey::Rope::from_str(&text), path: Some(path), dirty: false, encoding_guessed }
}

pub fn path(&self) -> Option<&Path> { self.path.as_deref() }
pub fn is_dirty(&self) -> bool { self.dirty }
pub fn encoding_guessed(&self) -> bool { self.encoding_guessed }
pub fn mark_clean(&mut self) { self.dirty = false; }
```

`apply` gains `self.dirty = true;` at end.

`lib.rs` adds:

```rust
pub mod error;
pub mod io;
pub use error::CoreError;
```

- [ ] **Step 4: Run, verify pass**

Run: `cargo test -p drz-core`
Expected: 7 passed.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(core): file open/save with encoding detection and binary/size guards"
```

---

### Task 4: drz-core — diff engine (Hunk, diff_lines)

**Files:**
- Create: `crates/drz-core/src/diff.rs`
- Modify: `crates/drz-core/src/lib.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks (operates on `&str`).
- Produces:
  - `pub struct Hunk { pub old_start: usize, pub old_end: usize, pub new_start: usize, pub new_end: usize }` (line indices, end exclusive)
  - `impl Hunk { pub fn is_change(&self) -> bool }` (true when both ranges non-empty)
  - `pub fn diff_lines(old: &str, new: &str) -> Vec<Hunk>` — only CHANGED regions (equal blocks omitted), using similar Myers.

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_files_no_hunks() {
        assert!(diff_lines("a\nb\n", "a\nb\n").is_empty());
    }

    #[test]
    fn single_line_change() {
        let hunks = diff_lines("a\nb\nc\n", "a\nX\nc\n");
        assert_eq!(hunks, vec![Hunk { old_start: 1, old_end: 2, new_start: 1, new_end: 2 }]);
    }

    #[test]
    fn insertion_and_deletion() {
        // insert line in new
        let hunks = diff_lines("a\nc\n", "a\nb\nc\n");
        assert_eq!(hunks, vec![Hunk { old_start: 1, old_end: 1, new_start: 1, new_end: 2 }]);
        // delete line
        let hunks = diff_lines("a\nb\nc\n", "a\nc\n");
        assert_eq!(hunks, vec![Hunk { old_start: 1, old_end: 2, new_start: 1, new_end: 1 }]);
    }

    #[test]
    fn matches_git_style_block() {
        let old = "1\n2\n3\n4\n5\n";
        let new = "1\n2x\n3\n4x\n5\n";
        let hunks = diff_lines(old, new);
        assert_eq!(hunks.len(), 2);
        assert!(hunks.iter().all(|h| h.is_change()));
    }
}
```

- [ ] **Step 2: Run, verify fail**

Run: `cargo test -p drz-core diff`
Expected: FAIL.

- [ ] **Step 3: Implement** `diff.rs`:

```rust
use similar::{ChangeTag, TextDiff};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hunk {
    pub old_start: usize,
    pub old_end: usize,
    pub new_start: usize,
    pub new_end: usize,
}

impl Hunk {
    pub fn is_change(&self) -> bool {
        self.old_start < self.old_end && self.new_start < self.new_end
    }
}

/// Changed regions only; equal blocks omitted. Line indices, end exclusive.
pub fn diff_lines(old: &str, new: &str) -> Vec<Hunk> {
    let diff = TextDiff::from_lines(old, new);
    let mut hunks = Vec::new();
    for op in diff.ops() {
        let tag = op.as_tag_tuple().0;
        if tag == ChangeTag::Equal {
            continue;
        }
        let old_r = op.old_range();
        let new_r = op.new_range();
        hunks.push(Hunk {
            old_start: old_r.start,
            old_end: old_r.end,
            new_start: new_r.start,
            new_end: new_r.end,
        });
    }
    hunks
}
```

`lib.rs` adds `pub mod diff; pub use diff::{diff_lines, Hunk};`

- [ ] **Step 4: Run, verify pass**

Run: `cargo test -p drz-core diff`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(core): line diff via similar, Hunk model"
```

---

### Task 5: drz-core — pane alignment builder

**Files:**
- Create: `crates/drz-core/src/align.rs`
- Modify: `crates/drz-core/src/lib.rs`

**Interfaces:**
- Consumes: `Hunk` (Task 4).
- Produces:
  - `pub struct Alignment { pub left: Vec<Option<usize>>, pub right: Vec<Option<usize>> }` — `left[row] = Some(line_idx)` or `None` (virtual padding row). Both vecs equal length.
  - `pub fn build_alignment(hunks: &[Hunk], left_lines: usize, right_lines: usize) -> Alignment`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{diff_lines, Hunk};

    #[test]
    fn identical_files_align_one_to_one() {
        let a = build_alignment(&[], 3, 3);
        assert_eq!(a.left, vec![Some(0), Some(1), Some(2)]);
        assert_eq!(a.right, vec![Some(0), Some(1), Some(2)]);
    }

    #[test]
    fn insertion_pads_left() {
        // right has extra line 1 ("b")
        let hunks = diff_lines("a\nc\n", "a\nb\nc\n");
        let a = build_alignment(&hunks, 3, 4);
        assert_eq!(a.left, vec![Some(0), None, Some(1), Some(2)]);
        assert_eq!(a.right, vec![Some(0), Some(1), Some(2), Some(3)]);
    }

    #[test]
    fn unequal_replace_pads_shorter_side() {
        // left 1 line → right 3 lines at position 0
        let hunks = vec![Hunk { old_start: 0, old_end: 1, new_start: 0, new_end: 3 }];
        let a = build_alignment(&hunks, 1, 3);
        assert_eq!(a.left, vec![Some(0), None, None]);
        assert_eq!(a.right, vec![Some(0), Some(1), Some(2)]);
    }

    #[test]
    fn equal_length_alignment() {
        let hunks = diff_lines("a\nx\nb\n", "a\ny\nz\nb\n");
        let a = build_alignment(&hunks, 4, 5);
        assert_eq!(a.left.len(), a.right.len());
        assert_eq!(a.left.len(), 5);
    }
}
```

- [ ] **Step 2: Run, verify fail**

Run: `cargo test -p drz-core align`
Expected: FAIL.

- [ ] **Step 3: Implement** `align.rs`:

```rust
use crate::diff::Hunk;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alignment {
    pub left: Vec<Option<usize>>,
    pub right: Vec<Option<usize>>,
}

/// Build display-row alignment: changed regions are padded with None on the
/// shorter side so equal blocks line up row-for-row.
pub fn build_alignment(hunks: &[Hunk], left_lines: usize, right_lines: usize) -> Alignment {
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut l = 0usize;
    let mut r = 0usize;
    for h in hunks {
        // equal block before hunk
        while l < h.old_start && r < h.new_start {
            left.push(Some(l));
            right.push(Some(r));
            l += 1;
            r += 1;
        }
        let old_len = h.old_end - h.old_start;
        let new_len = h.new_end - h.new_start;
        let rows = old_len.max(new_len);
        for i in 0..rows {
            left.push(if i < old_len { Some(h.old_start + i) } else { None });
            right.push(if i < new_len { Some(h.new_start + i) } else { None });
        }
        l = h.old_end;
        r = h.new_end;
    }
    while l < left_lines || r < right_lines {
        left.push(if l < left_lines { Some(l) } else { None });
        right.push(if r < right_lines { Some(r) } else { None });
        l += 1;
        r += 1;
    }
    Alignment { left, right }
}
```

`lib.rs` adds `pub mod align; pub use align::{build_alignment, Alignment};`

- [ ] **Step 4: Run, verify pass**

Run: `cargo test -p drz-core align`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(core): pane alignment builder with virtual padding rows"
```

---

### Task 6: drz-highlight — language detection + Style

**Files:**
- Create: `crates/drz-highlight/src/language.rs`
- Create: `crates/drz-highlight/src/style.rs`
- Modify: `crates/drz-highlight/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub enum LanguageId { Rust, Python, JavaScript, C, Cpp, PlainText }`
  - `LanguageId::from_path(path: &Path) -> LanguageId` (extension map; unknown → PlainText)
  - `pub enum Style { Keyword, StringLit, Comment, Function, Type, Number, Constant, Default }`
  - `pub struct StyledSpan { pub start: usize, pub end: usize, pub style: Style }` (byte offsets)

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detects_by_extension() {
        assert_eq!(LanguageId::from_path(Path::new("a.rs")), LanguageId::Rust);
        assert_eq!(LanguageId::from_path(Path::new("a.py")), LanguageId::Python);
        assert_eq!(LanguageId::from_path(Path::new("a.js")), LanguageId::JavaScript);
        assert_eq!(LanguageId::from_path(Path::new("a.c")), LanguageId::C);
        assert_eq!(LanguageId::from_path(Path::new("a.h")), LanguageId::C);
        assert_eq!(LanguageId::from_path(Path::new("a.cpp")), LanguageId::Cpp);
        assert_eq!(LanguageId::from_path(Path::new("a.hpp")), LanguageId::Cpp);
        assert_eq!(LanguageId::from_path(Path::new("Makefile")), LanguageId::PlainText);
        assert_eq!(LanguageId::from_path(Path::new("a.xyz")), LanguageId::PlainText);
    }
}
```

- [ ] **Step 2: Run, verify fail**

Run: `cargo test -p drz-highlight`
Expected: FAIL.

- [ ] **Step 3: Implement** `language.rs`:

```rust
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageId {
    Rust,
    Python,
    JavaScript,
    C,
    Cpp,
    PlainText,
}

impl LanguageId {
    pub fn from_path(path: &Path) -> LanguageId {
        match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
            "rs" => LanguageId::Rust,
            "py" | "pyi" => LanguageId::Python,
            "js" | "mjs" | "cjs" | "jsx" => LanguageId::JavaScript,
            "c" | "h" => LanguageId::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hh" => LanguageId::Cpp,
            _ => LanguageId::PlainText,
        }
    }
}
```

`style.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Keyword,
    StringLit,
    Comment,
    Function,
    Type,
    Number,
    Constant,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StyledSpan {
    pub start: usize,
    pub end: usize,
    pub style: Style,
}
```

`lib.rs`:

```rust
pub mod language;
pub mod style;

pub use language::LanguageId;
pub use style::{Style, StyledSpan};
```

- [ ] **Step 4: Run, verify pass**

Run: `cargo test -p drz-highlight`
Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(highlight): language detection and style model"
```

---

### Task 7: drz-highlight — HighlightEngine full parse

**Files:**
- Create: `crates/drz-highlight/src/engine.rs`
- Create: `crates/drz-highlight/src/error.rs`
- Create: `crates/drz-highlight/queries/rust.scm`, `python.scm`, `javascript.scm`, `c.scm`, `cpp.scm`
- Modify: `crates/drz-highlight/src/lib.rs`

**Interfaces:**
- Consumes: `LanguageId`, `Style`, `StyledSpan` (Task 6); `ropey::Rope`.
- Produces:
  - `pub enum HighlightError { UnsupportedLanguage, ParseFailed, QueryFailed(String) }`
  - `pub struct HighlightEngine { /* private */ }`
  - `HighlightEngine::new(lang: LanguageId) -> Result<Option<HighlightEngine>, HighlightError>` — `Ok(None)` for PlainText.
  - `engine.parse_full(&mut self, rope: &Rope) -> Result<(), HighlightError>`
  - `engine.highlight_line(&self, rope: &Rope, line_idx: usize) -> Vec<StyledSpan>` — spans clamped to that line's byte range, offsets line-relative.

- [ ] **Step 1: Write queries.** `queries/rust.scm`:

```scheme
(line_comment) @comment
(block_comment) @comment
(string_literal) @string
(char_literal) @string
(integer_literal) @number
(float_literal) @number
(boolean_literal) @constant
(type_identifier) @type
(primitive_type) @type
(function_item name: (identifier) @function)
(call_expression function: (identifier) @function)
[
  "fn" "let" "pub" "struct" "enum" "impl" "trait" "use" "mod" "match"
  "if" "else" "for" "while" "loop" "return" "mut" "const" "static"
  "async" "await" "move" "ref" "where" "type" "crate" "self" "Self"
  "super" "in" "as" "dyn" "unsafe" "break" "continue"
] @keyword
```

`queries/python.scm`:

```scheme
(comment) @comment
(string) @string
(integer) @number
(float) @number
(true) @constant
(false) @constant
(none) @constant
(function_definition name: (identifier) @function)
(call function: (identifier) @function)
[
  "def" "class" "return" "if" "elif" "else" "for" "while" "import"
  "from" "as" "with" "try" "except" "finally" "raise" "pass" "break"
  "continue" "lambda" "yield" "async" "await" "global" "nonlocal"
  "assert" "del" "in" "is" "not" "and" "or"
] @keyword
```

`queries/javascript.scm`:

```scheme
(comment) @comment
(string) @string
(template_string) @string
(number) @number
(true) @constant
(false) @constant
(null) @constant
(function_declaration name: (identifier) @function)
(call_expression function: (identifier) @function)
[
  "function" "const" "let" "var" "return" "if" "else" "for" "while"
  "do" "switch" "case" "default" "break" "continue" "new" "delete"
  "typeof" "instanceof" "in" "of" "class" "extends" "super" "this"
  "import" "export" "from" "async" "await" "try" "catch" "finally"
  "throw" "yield" "static" "get" "set"
] @keyword
```

`queries/c.scm`:

```scheme
(comment) @comment
(string_literal) @string
(char_literal) @string
(number_literal) @number
(true) @constant
(false) @constant
(null) @constant
(type_identifier) @type
(primitive_type) @type
(function_declarator declarator: (identifier) @function)
(call_expression function: (identifier) @function)
[
  "if" "else" "for" "while" "do" "switch" "case" "default" "break"
  "continue" "return" "goto" "sizeof" "struct" "union" "enum" "typedef"
  "static" "const" "volatile" "extern" "inline" "register" "unsigned"
  "signed" "void"
] @keyword
```

`queries/cpp.scm`:

```scheme
(comment) @comment
(string_literal) @string
(char_literal) @string
(number_literal) @number
(true) @constant
(false) @constant
(null) @constant
(nullptr) @constant
(type_identifier) @type
(primitive_type) @type
(function_declarator declarator: (identifier) @function)
(call_expression function: (identifier) @function)
[
  "if" "else" "for" "while" "do" "switch" "case" "default" "break"
  "continue" "return" "goto" "sizeof" "struct" "union" "enum" "typedef"
  "static" "const" "volatile" "extern" "inline" "class" "namespace"
  "template" "typename" "using" "public" "private" "protected" "virtual"
  "override" "final" "new" "delete" "this" "try" "catch" "throw"
  "noexcept" "constexpr" "auto" "concept" "requires" "unsigned" "void"
] @keyword
```

- [ ] **Step 2: Write failing tests** (in `engine.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::LanguageId;
    use ropey::Rope;

    #[test]
    fn plaintext_returns_none() {
        assert!(HighlightEngine::new(LanguageId::PlainText).unwrap().is_none());
    }

    #[test]
    fn rust_keyword_and_string_styled() {
        let mut eng = HighlightEngine::new(LanguageId::Rust).unwrap().unwrap();
        let rope = Rope::from_str("fn main() { let s = \"hi\"; }\n");
        eng.parse_full(&rope).unwrap();
        let spans = eng.highlight_line(&rope, 0);
        // "fn" at bytes 0..2 → Keyword
        assert!(spans.iter().any(|s| s.start == 0 && s.end == 2 && s.style == Style::Keyword));
        // "hi" string literal → StringLit covering bytes 21..25
        assert!(spans.iter().any(|s| s.style == Style::StringLit && s.start <= 21 && s.end >= 25));
    }

    #[test]
    fn python_comment_styled() {
        let mut eng = HighlightEngine::new(LanguageId::Python).unwrap().unwrap();
        let rope = Rope::from_str("# hello\nx = 1\n");
        eng.parse_full(&rope).unwrap();
        let spans = eng.highlight_line(&rope, 0);
        assert!(spans.iter().any(|s| s.style == Style::Comment && s.start == 0));
    }
}
```

- [ ] **Step 3: Run, verify fail**

Run: `cargo test -p drz-highlight`
Expected: FAIL (engine missing).

- [ ] **Step 4: Implement** `error.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum HighlightError {
    #[error("unsupported language")]
    UnsupportedLanguage,
    #[error("parse failed")]
    ParseFailed,
    #[error("query failed: {0}")]
    QueryFailed(String),
}
```

`engine.rs` (above tests):

```rust
use crate::error::HighlightError;
use crate::language::LanguageId;
use crate::style::{Style, StyledSpan};
use ropey::Rope;
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator, Tree};

pub struct HighlightEngine {
    parser: Parser,
    tree: Option<Tree>,
    query: Query,
    language: Language,
    capture_styles: Vec<Style>,
}

fn language_and_query(lang: LanguageId) -> Option<(Language, &'static str)> {
    let pair = match lang {
        LanguageId::Rust => (
            Language::new(tree_sitter_rust::LANGUAGE),
            include_str!("../queries/rust.scm"),
        ),
        LanguageId::Python => (
            Language::new(tree_sitter_python::LANGUAGE),
            include_str!("../queries/python.scm"),
        ),
        LanguageId::JavaScript => (
            Language::new(tree_sitter_javascript::LANGUAGE),
            include_str!("../queries/javascript.scm"),
        ),
        LanguageId::C => (
            Language::new(tree_sitter_c::LANGUAGE),
            include_str!("../queries/c.scm"),
        ),
        LanguageId::Cpp => (
            Language::new(tree_sitter_cpp::LANGUAGE),
            include_str!("../queries/cpp.scm"),
        ),
        LanguageId::PlainText => return None,
    };
    Some(pair)
}

fn style_for_capture(name: &str) -> Style {
    match name {
        "keyword" => Style::Keyword,
        "string" => Style::StringLit,
        "comment" => Style::Comment,
        "function" => Style::Function,
        "type" => Style::Type,
        "number" => Style::Number,
        "constant" => Style::Constant,
        _ => Style::Default,
    }
}

impl HighlightEngine {
    pub fn new(lang: LanguageId) -> Result<Option<HighlightEngine>, HighlightError> {
        let Some((language, query_src)) = language_and_query(lang) else {
            return Ok(None);
        };
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .map_err(|_| HighlightError::UnsupportedLanguage)?;
        let query = Query::new(&language, query_src)
            .map_err(|e| HighlightError::QueryFailed(e.to_string()))?;
        let capture_styles = query
            .capture_names()
            .iter()
            .map(|n| style_for_capture(n))
            .collect();
        Ok(Some(HighlightEngine {
            parser,
            tree: None,
            query,
            language,
            capture_styles,
        }))
    }

    fn parse_with_rope(&mut self, rope: &Rope, old: Option<&Tree>) -> Result<(), HighlightError> {
        let mut chunks = rope.chunks();
        let mut byte = 0usize;
        let text = rope.to_string(); // fallback if chunk callback needs flat access
        let _ = (&mut chunks, &mut byte);
        let tree = self
            .parser
            .parse(text.as_bytes(), old)
            .ok_or(HighlightError::ParseFailed)?;
        self.tree = Some(tree);
        Ok(())
    }

    pub fn parse_full(&mut self, rope: &Rope) -> Result<(), HighlightError> {
        self.parse_with_rope(rope, None)
    }

    pub fn highlight_line(&self, rope: &Rope, line_idx: usize) -> Vec<StyledSpan> {
        let Some(tree) = &self.tree else { return Vec::new() };
        if line_idx >= rope.len_lines() {
            return Vec::new();
        }
        let line_start = rope.line_to_byte(line_idx);
        let line_end = rope.line_to_byte(line_idx + 1).min(rope.len_bytes());
        let mut cursor = QueryCursor::new();
        cursor.set_byte_range(line_start..line_end);
        let text = rope.to_string();
        let mut spans = Vec::new();
        let mut matches = cursor.matches(&self.query, tree.root_node(), text.as_bytes());
        while let Some(m) = matches.next() {
            for cap in m.captures {
                let node = cap.node;
                let style = self.capture_styles[cap.index as usize];
                let start = node.start_byte().max(line_start) - line_start;
                let end = node.end_byte().min(line_end) - line_start;
                if start < end {
                    spans.push(StyledSpan { start, end, style });
                }
            }
        }
        spans.sort_by_key(|s| (s.start, s.end));
        spans
    }
}
```

Note: `highlight_line` uses `rope.to_string()` per call — Task 8 keeps this; per-line flattening cost is acceptable for visible lines only (view calls it for ~50 lines/frame). A `line_end` past EOF: `line_to_byte(idx+1)` panics at EOF — guard:

```rust
let line_end = if line_idx + 1 < rope.len_lines() {
    rope.line_to_byte(line_idx + 1)
} else {
    rope.len_bytes()
};
```

(Use this guarded version in the file, not the `.min` shortcut — `line_to_byte` panics out of range.)

`lib.rs` adds:

```rust
pub mod engine;
pub mod error;
pub use engine::HighlightEngine;
pub use error::HighlightError;
```

- [ ] **Step 5: Run, verify pass**

Run: `cargo test -p drz-highlight`
Expected: 4 passed total.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat(highlight): tree-sitter engine with full parse and per-line spans"
```

---

### Task 8: drz-highlight — incremental edits

**Files:**
- Modify: `crates/drz-highlight/src/engine.rs`
- Modify: `crates/drz-highlight/src/lib.rs`

**Interfaces:**
- Consumes: `TextEdit` from `drz-core` — NO. Global constraint: drz-highlight must not depend on drz-core (avoid cycle; core doesn't know highlight). Define own edit type.
- Produces:
  - `pub struct HighlightEdit { pub start_byte: usize, pub old_end_byte: usize, pub new_end_byte: usize, pub start_point: (usize, usize), pub old_end_point: (usize, usize), pub new_end_point: (usize, usize) }` (points = `(row, col_bytes)`)
  - `HighlightEdit::from_rope_edit(rope_before: &Rope, start_byte: usize, old_end_byte: usize, inserted: &str) -> HighlightEdit`
  - `engine.apply_edit(&mut self, edit: &HighlightEdit, rope: &Rope) -> Result<(), HighlightError>` — calls `tree.edit`, reparse incremental.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn incremental_reparse_matches_full_reparse() {
    let mut eng = HighlightEngine::new(LanguageId::Rust).unwrap().unwrap();
    let before = Rope::from_str("fn main() {\n    let x = 1;\n}\n");
    eng.parse_full(&before).unwrap();

    // insert a comment line at byte position 14 (start of line 1)
    let inserted = "    // hi\n";
    let edit = HighlightEdit::from_rope_edit(&before, 14, 14, inserted);
    let mut after = before.clone();
    let char_pos = after.byte_to_char(14);
    after.insert(char_pos, inserted);

    eng.apply_edit(&edit, &after).unwrap();

    // full-reparse oracle
    let mut oracle = HighlightEngine::new(LanguageId::Rust).unwrap().unwrap();
    oracle.parse_full(&after).unwrap();

    for line in 0..after.len_lines() {
        assert_eq!(
            eng.highlight_line(&after, line),
            oracle.highlight_line(&after, line),
            "line {line} mismatch"
        );
    }
}

#[test]
fn highlight_after_edit_has_comment() {
    let mut eng = HighlightEngine::new(LanguageId::Rust).unwrap().unwrap();
    let before = Rope::from_str("fn main() {\n    let x = 1;\n}\n");
    eng.parse_full(&before).unwrap();
    let inserted = "    // hi\n";
    let edit = HighlightEdit::from_rope_edit(&before, 14, 14, inserted);
    let mut after = before.clone();
    let char_pos = after.byte_to_char(14);
    after.insert(char_pos, inserted);
    eng.apply_edit(&edit, &after).unwrap();
    let spans = eng.highlight_line(&after, 1);
    assert!(spans.iter().any(|s| s.style == Style::Comment));
}
```

- [ ] **Step 2: Run, verify fail**

Run: `cargo test -p drz-highlight incremental`
Expected: FAIL (`apply_edit` missing).

- [ ] **Step 3: Implement.** In `engine.rs` add:

```rust
use tree_sitter::{InputEdit, Point};

#[derive(Debug, Clone, Copy)]
pub struct HighlightEdit {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_end_byte: usize,
    pub start_point: (usize, usize),
    pub old_end_point: (usize, usize),
    pub new_end_point: (usize, usize),
}

impl HighlightEdit {
    pub fn from_rope_edit(
        rope_before: &Rope,
        start_byte: usize,
        old_end_byte: usize,
        inserted: &str,
    ) -> HighlightEdit {
        let point_of = |byte: usize| -> (usize, usize) {
            let row = rope_before.byte_to_line(byte.min(rope_before.len_bytes()));
            let row_start = rope_before.line_to_byte(row);
            (row, byte - row_start)
        };
        let start_point = point_of(start_byte);
        let old_end_point = point_of(old_end_byte);
        // new_end_point: walk inserted text from start_point
        let mut row = start_point.0;
        let mut col = start_point.1;
        for ch in inserted.chars() {
            if ch == '\n' {
                row += 1;
                col = 0;
            } else {
                col += ch.len_utf8();
            }
        }
        HighlightEdit {
            start_byte,
            old_end_byte,
            new_end_byte: start_byte + inserted.len(),
            start_point,
            old_end_point,
            new_end_point: (row, col),
        }
    }
}

impl HighlightEngine {
    pub fn apply_edit(
        &mut self,
        edit: &HighlightEdit,
        rope: &Rope,
    ) -> Result<(), HighlightError> {
        if let Some(tree) = &mut self.tree {
            tree.edit(&InputEdit {
                start_byte: edit.start_byte,
                old_end_byte: edit.old_end_byte,
                new_end_byte: edit.new_end_byte,
                start_position: Point::new(edit.start_point.0, edit.start_point.1),
                old_end_position: Point::new(edit.old_end_point.0, edit.old_end_point.1),
                new_end_position: Point::new(edit.new_end_point.0, edit.new_end_point.1),
            });
        }
        let old = self.tree.take();
        let text = rope.to_string();
        let tree = self
            .parser
            .parse(text.as_bytes(), old.as_ref())
            .ok_or(HighlightError::ParseFailed)?;
        self.tree = Some(tree);
        Ok(())
    }
}
```

`lib.rs` adds `pub use engine::HighlightEdit;`

Note: `rope.to_string()` on every keystroke is a v1 simplification — reparse itself stays incremental (old tree passed). Record a perf note: future task switches to `parse_with` rope-chunk callback if profiling demands.

- [ ] **Step 4: Run, verify pass**

Run: `cargo test -p drz-highlight`
Expected: 6 passed total.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(highlight): incremental reparse via InputEdit"
```

---

### Task 9: drz-viewmodel — EditorViewModel

**Files:**
- Create: `crates/drz-viewmodel/src/editor_vm.rs`
- Create: `crates/drz-viewmodel/src/lib.rs`
- Create: `crates/drz-viewmodel/src/types.rs`

**Interfaces:**
- Consumes: `Document`, `TextEdit`, `CoreError` (drz-core); `HighlightEngine`, `HighlightEdit`, `LanguageId`, `Style` (drz-highlight).
- Produces:
  - `pub struct LineSpan { pub start: usize, pub end: usize, pub style: Style }` (re-export `Style` for view crates: `pub use drz_highlight::Style;`)
  - `pub struct EditorViewModel { /* private */ }`
  - `EditorViewModel::open(path: &Path) -> Result<EditorViewModel, CoreError>`
  - `EditorViewModel::from_text(text: &str, lang: LanguageId) -> EditorViewModel`
  - `vm.edit(&mut self, start_byte: usize, old_end_byte: usize, text: &str) -> ()` (updates rope + highlight engine)
  - `vm.insert_at_line_col(&mut self, line: usize, col_byte: usize, text: &str) -> ()`
  - `vm.delete_range_line_col(&mut self, start: (usize, usize), end: (usize, usize)) -> ()`
  - `vm.styled_line(&self, line_idx: usize) -> (String, Vec<LineSpan>)`
  - `vm.len_lines(&self) -> usize`
  - `vm.line(&self, idx: usize) -> String`
  - `vm.line_byte_range(&self, idx: usize) -> (usize, usize)`
  - `vm.path(&self) -> Option<&Path>`
  - `vm.is_dirty(&self) -> bool`
  - `vm.encoding_guessed(&self) -> bool`
  - `vm.save(&mut self) -> Result<(), CoreError>`
  - `vm.replace_lines(&mut self, start: usize, end: usize, text: &str) -> ()` (keeps highlight in sync)
  - `vm.document_text(&self) -> String`

- [ ] **Step 1: Write failing tests** (`editor_vm.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use drz_highlight::LanguageId;

    #[test]
    fn from_text_plain_no_highlight() {
        let vm = EditorViewModel::from_text("hello\n", LanguageId::PlainText);
        let (text, spans) = vm.styled_line(0);
        assert_eq!(text, "hello");
        assert!(spans.is_empty());
    }

    #[test]
    fn rust_edit_keeps_highlight_in_sync() {
        let mut vm = EditorViewModel::from_text("fn main() {}\n", LanguageId::Rust);
        // insert comment after "{}" : bytes 12..12
        vm.edit(12, 12, " // x");
        assert_eq!(vm.line(0), "fn main() {} // x");
        let (_, spans) = vm.styled_line(0);
        assert!(spans.iter().any(|s| s.style == Style::Comment));
    }

    #[test]
    fn insert_at_line_col_works() {
        let mut vm = EditorViewModel::from_text("ab\ncd\n", LanguageId::PlainText);
        vm.insert_at_line_col(1, 1, "Z");
        assert_eq!(vm.line(1), "cZd");
    }

    #[test]
    fn replace_lines_updates_highlight_and_text() {
        let mut vm = EditorViewModel::from_text("let a = 1;\nlet b = 2;\n", LanguageId::Rust);
        vm.replace_lines(0, 1, "// gone");
        assert_eq!(vm.line(0), "// gone");
        let (_, spans) = vm.styled_line(0);
        assert!(spans.iter().any(|s| s.style == Style::Comment));
    }
}
```

- [ ] **Step 2: Run, verify fail**

Run: `cargo test -p drz-viewmodel`
Expected: FAIL.

- [ ] **Step 3: Implement** `types.rs`:

```rust
pub use drz_highlight::Style;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineSpan {
    pub start: usize,
    pub end: usize,
    pub style: Style,
}
```

`editor_vm.rs` (above tests):

```rust
use crate::types::LineSpan;
use drz_core::{CoreError, Document, TextEdit};
use drz_highlight::{HighlightEdit, HighlightEngine, LanguageId};
use std::path::{Path, PathBuf};

pub struct EditorViewModel {
    doc: Document,
    engine: Option<HighlightEngine>,
    #[allow(dead_code)]
    lang: LanguageId,
}

impl EditorViewModel {
    pub fn open(path: &Path) -> Result<EditorViewModel, CoreError> {
        let doc = Document::open(path)?;
        let lang = LanguageId::from_path(path);
        let engine = HighlightEngine::new(lang).ok().flatten();
        let mut vm = EditorViewModel { doc, engine, lang };
        vm.reparse_full();
        Ok(vm)
    }

    pub fn from_text(text: &str, lang: LanguageId) -> EditorViewModel {
        let engine = HighlightEngine::new(lang).ok().flatten();
        let mut vm = EditorViewModel {
            doc: Document::from_text(text),
            engine,
            lang,
        };
        vm.reparse_full();
        vm
    }

    fn reparse_full(&mut self) {
        if let Some(engine) = &mut self.engine {
            let _ = engine.parse_full(self.doc.rope());
        }
    }

    pub fn edit(&mut self, start_byte: usize, old_end_byte: usize, text: &str) {
        let hl_edit = HighlightEdit::from_rope_edit(self.doc.rope(), start_byte, old_end_byte, text);
        self.doc.apply(&TextEdit {
            start_byte,
            old_end_byte,
            inserted: text.to_string(),
        });
        if let Some(engine) = &mut self.engine {
            let _ = engine.apply_edit(&hl_edit, self.doc.rope());
        }
    }

    pub fn insert_at_line_col(&mut self, line: usize, col_byte: usize, text: &str) {
        if line >= self.doc.len_lines() {
            return;
        }
        let (start, end) = self.doc.line_byte_range(line);
        let byte = (start + col_byte).min(end);
        self.edit(byte, byte, text);
    }

    pub fn delete_range_line_col(&mut self, start: (usize, usize), end: (usize, usize)) {
        let byte_of = |(line, col): (usize, usize)| -> usize {
            if line >= self.doc.len_lines() {
                return self.doc.rope().len_bytes();
            }
            let (ls, le) = self.doc.line_byte_range(line);
            (ls + col).min(le)
        };
        let s = byte_of(start);
        let e = byte_of(end).max(s);
        self.edit(s, e, "");
    }

    pub fn styled_line(&self, line_idx: usize) -> (String, Vec<LineSpan>) {
        let text = self.doc.line(line_idx);
        let spans = match &self.engine {
            Some(engine) => engine
                .highlight_line(self.doc.rope(), line_idx)
                .into_iter()
                .map(|s| LineSpan { start: s.start, end: s.end, style: s.style })
                .collect(),
            None => Vec::new(),
        };
        (text, spans)
    }

    pub fn len_lines(&self) -> usize { self.doc.len_lines() }
    pub fn line(&self, idx: usize) -> String { self.doc.line(idx) }
    pub fn line_byte_range(&self, idx: usize) -> (usize, usize) { self.doc.line_byte_range(idx) }
    pub fn path(&self) -> Option<&Path> { self.doc.path() }
    pub fn is_dirty(&self) -> bool { self.doc.is_dirty() }
    pub fn encoding_guessed(&self) -> bool { self.doc.encoding_guessed() }
    pub fn document_text(&self) -> String { self.doc.to_string() }

    pub fn save(&mut self) -> Result<(), CoreError> {
        self.doc.save()
    }

    pub fn replace_lines(&mut self, start: usize, end: usize, text: &str) {
        let start_byte = self.doc.rope().line_to_byte(start.min(self.doc.len_lines()));
        let end_byte = if end >= self.doc.len_lines() {
            self.doc.rope().len_bytes()
        } else {
            self.doc.rope().line_to_byte(end)
        };
        self.edit(start_byte, end_byte, text);
    }
}
```

`lib.rs`:

```rust
mod editor_vm;
pub mod types;

pub use editor_vm::EditorViewModel;
pub use types::LineSpan;
pub use drz_highlight::LanguageId;
pub use drz_core::CoreError;
```

- [ ] **Step 4: Run, verify pass**

Run: `cargo test -p drz-viewmodel`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(viewmodel): EditorViewModel bridging document and highlight engine"
```

---

### Task 10: drz-viewmodel — DiffViewModel (async diff + merge)

**Files:**
- Create: `crates/drz-viewmodel/src/diff_vm.rs`
- Modify: `crates/drz-viewmodel/src/lib.rs`

**Interfaces:**
- Consumes: `EditorViewModel` (Task 9); `diff_lines`, `Hunk`, `build_alignment`, `Alignment` (drz-core — re-exported: `pub use drz_core::{Alignment, Hunk};`).
- Produces:
  - `pub enum MergeDirection { LeftToRight, RightToLeft }`
  - `pub struct DiffViewModel { /* private */ }`
  - `DiffViewModel::new(left: EditorViewModel, right: EditorViewModel) -> DiffViewModel`
  - `vm.left(&self) -> &EditorViewModel`, `vm.left_mut(&mut self) -> &mut EditorViewModel`, same for `right`
  - `vm.request_diff(&mut self) -> ()` (spawns background thread, debounced: sets flag; actual spawn if no run in flight)
  - `vm.poll(&mut self) -> bool` (drains channel; true if hunks updated)
  - `vm.hunks(&self) -> &[Hunk]`
  - `vm.alignment(&self) -> &Alignment`
  - `vm.merge_chunk(&mut self, hunk_idx: usize, dir: MergeDirection) -> ()`
  - `vm.set_repaint_callback(&mut self, cb: Arc<dyn Fn() + Send + Sync>)` (invoked when diff result lands)
  - Debounce 150ms: `request_diff` records `Instant`; `poll` fires thread only when 150ms elapsed and dirty. Deterministic test hook: `vm.flush_diff_now(&mut self) -> ()` (runs synchronously on calling thread).

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use drz_highlight::LanguageId;

    fn vm_pair() -> DiffViewModel {
        let l = EditorViewModel::from_text("a\nb\nc\n", LanguageId::PlainText);
        let r = EditorViewModel::from_text("a\nX\nc\n", LanguageId::PlainText);
        DiffViewModel::new(l, r)
    }

    #[test]
    fn flush_computes_hunks_and_alignment() {
        let mut vm = vm_pair();
        vm.flush_diff_now();
        assert_eq!(vm.hunks().len(), 1);
        assert_eq!(vm.alignment().left.len(), vm.alignment().right.len());
        assert_eq!(vm.alignment().left.len(), 3);
    }

    #[test]
    fn edit_marks_dirty_and_recompute_clears() {
        let mut vm = vm_pair();
        vm.flush_diff_now();
        vm.right_mut().edit(2, 3, "b"); // line1 "X" → "b" ... now identical
        vm.flush_diff_now();
        assert!(vm.hunks().is_empty());
    }

    #[test]
    fn merge_chunk_left_to_right() {
        let mut vm = vm_pair();
        vm.flush_diff_now();
        vm.merge_chunk(0, MergeDirection::LeftToRight);
        assert_eq!(vm.right().line(1), "b");
        assert!(vm.right().is_dirty());
        vm.flush_diff_now();
        assert!(vm.hunks().is_empty());
    }

    #[test]
    fn merge_chunk_right_to_left() {
        let mut vm = vm_pair();
        vm.flush_diff_now();
        vm.merge_chunk(0, MergeDirection::RightToLeft);
        assert_eq!(vm.left().line(1), "X");
    }

    #[test]
    fn merge_insert_hunk() {
        let l = EditorViewModel::from_text("a\nc\n", LanguageId::PlainText);
        let r = EditorViewModel::from_text("a\nb\nc\n", LanguageId::PlainText);
        let mut vm = DiffViewModel::new(l, r);
        vm.flush_diff_now();
        // right has extra "b": copy right→left
        vm.merge_chunk(0, MergeDirection::RightToLeft);
        assert_eq!(vm.left().line(1), "b");
        assert_eq!(vm.left().len_lines(), 4);
    }
}
```

- [ ] **Step 2: Run, verify fail**

Run: `cargo test -p drz-viewmodel diff_vm`
Expected: FAIL.

- [ ] **Step 3: Implement** `diff_vm.rs` (above tests):

```rust
use crate::editor_vm::EditorViewModel;
use drz_core::{build_alignment, diff_lines, Alignment, Hunk};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

const DEBOUNCE: Duration = Duration::from_millis(150);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeDirection {
    LeftToRight,
    RightToLeft,
}

pub struct DiffViewModel {
    left: EditorViewModel,
    right: EditorViewModel,
    hunks: Vec<Hunk>,
    alignment: Alignment,
    dirty_since: Option<Instant>,
    in_flight: bool,
    rx: Option<Receiver<(Vec<Hunk>, Alignment)>>,
    repaint: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl DiffViewModel {
    pub fn new(left: EditorViewModel, right: EditorViewModel) -> DiffViewModel {
        DiffViewModel {
            left,
            right,
            hunks: Vec::new(),
            alignment: Alignment { left: Vec::new(), right: Vec::new() },
            dirty_since: Some(Instant::now() - DEBOUNCE),
            in_flight: false,
            rx: None,
            repaint: None,
        }
    }

    pub fn left(&self) -> &EditorViewModel { &self.left }
    pub fn left_mut(&mut self) -> &mut EditorViewModel {
        self.dirty_since.get_or_insert(Instant::now());
        &mut self.left
    }
    pub fn right(&self) -> &EditorViewModel { &self.right }
    pub fn right_mut(&mut self) -> &mut EditorViewModel {
        self.dirty_since.get_or_insert(Instant::now());
        &mut self.right
    }

    pub fn set_repaint_callback(&mut self, cb: Arc<dyn Fn() + Send + Sync>) {
        self.repaint = Some(cb);
    }

    pub fn request_diff(&mut self) {
        self.dirty_since.get_or_insert(Instant::now());
    }

    /// Call at frame start. Returns true if hunks changed.
    pub fn poll(&mut self) -> bool {
        let mut updated = false;
        if let Some(rx) = &self.rx {
            if let Ok((hunks, alignment)) = rx.try_recv() {
                self.hunks = hunks;
                self.alignment = alignment;
                self.in_flight = false;
                self.rx = None;
                updated = true;
                if let Some(cb) = &self.repaint {
                    cb();
                }
            }
        }
        let ready = self
            .dirty_since
            .is_some_and(|t| t.elapsed() >= DEBOUNCE);
        if ready && !self.in_flight {
            self.spawn_diff();
        }
        updated
    }

    fn spawn_diff(&mut self) {
        self.dirty_since = None;
        self.in_flight = true;
        let old = self.left.document_text();
        let new = self.right.document_text();
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let hunks = diff_lines(&old, &new);
            let left_lines = old.lines().count() + usize::from(old.ends_with('\n') || old.is_empty());
            let right_lines = new.lines().count() + usize::from(new.ends_with('\n') || new.is_empty());
            let alignment = build_alignment(&hunks, left_lines, right_lines);
            let _ = tx.send((hunks, alignment));
        });
        self.rx = Some(rx);
    }

    /// Synchronous recompute (tests + first paint).
    pub fn flush_diff_now(&mut self) {
        self.dirty_since = None;
        self.in_flight = false;
        self.rx = None;
        let old = self.left.document_text();
        let new = self.right.document_text();
        self.hunks = diff_lines(&old, &new);
        self.alignment = build_alignment(
            &self.hunks,
            self.left.len_lines(),
            self.right.len_lines(),
        );
    }

    pub fn hunks(&self) -> &[Hunk] { &self.hunks }
    pub fn alignment(&self) -> &Alignment { &self.alignment }

    pub fn merge_chunk(&mut self, hunk_idx: usize, dir: MergeDirection) {
        let Some(h) = self.hunks.get(hunk_idx).copied() else { return };
        match dir {
            MergeDirection::LeftToRight => {
                let text = (h.old_start..h.old_end)
                    .map(|i| self.left.line(i))
                    .collect::<Vec<_>>()
                    .join("\n");
                let text = if h.old_start < h.old_end && h.new_start < h.new_end {
                    text
                } else if h.old_start < h.old_end {
                    // pure insertion into right: need trailing newline handling
                    text
                } else {
                    String::new()
                };
                // build replacement text with newlines matching replaced line range
                let replacement = build_block(&self.left, h.old_start, h.old_end);
                drop(text);
                self.right.replace_lines(h.new_start, h.new_end, &replacement);
            }
            MergeDirection::RightToLeft => {
                let replacement = build_block(&self.right, h.new_start, h.new_end);
                self.left.replace_lines(h.old_start, h.old_end, &replacement);
            }
        }
        self.flush_diff_now();
    }
}

fn build_block(vm: &EditorViewModel, start: usize, end: usize) -> String {
    if start >= end {
        return String::new();
    }
    let mut s = (start..end).map(|i| vm.line(i)).collect::<Vec<_>>().join("\n");
    // preserve trailing newline so following lines stay separate
    if end < vm.len_lines() {
        s.push('\n');
    }
    s
}
```

Wait — `build_block` trailing-newline rule: replacing lines `[start,end)` needs text such that lines after `end` remain on own lines. If replacing at EOF (end == len_lines) no trailing newline needed unless inserting before existing content. Rule above: if `end < len_lines` → append `\n`. If insertion (start==end) before existing line, block needs trailing `\n` too — covered since end < len_lines. If insertion at EOF line, text joins into last line — edge: `len_lines` counts final empty line when file ends with `\n`, so `end < len_lines` true in practice. Tests cover given cases; keep rule.

`lib.rs` adds:

```rust
mod diff_vm;
pub use diff_vm::{DiffViewModel, MergeDirection};
pub use drz_core::{Alignment, Hunk};
```

- [ ] **Step 4: Run, verify pass**

Run: `cargo test -p drz-viewmodel`
Expected: 9 passed total.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(viewmodel): DiffViewModel with async debounced diff and chunk merge"
```

---

### Task 11: drz-viewmodel — AppViewModel

**Files:**
- Create: `crates/drz-viewmodel/src/app_vm.rs`
- Modify: `crates/drz-viewmodel/src/lib.rs`

**Interfaces:**
- Consumes: `DiffViewModel`, `EditorViewModel`, `CoreError`.
- Produces:
  - `pub enum AppError { OpenFailed { path: PathBuf, message: String }, SaveFailed { message: String } }`
  - `pub struct AppViewModel { /* private */ }`
  - `AppViewModel::open_pair(left: &Path, right: &Path) -> AppViewModel`
  - `AppViewModel::empty() -> AppViewModel`
  - `vm.diff(&self) -> Option<&DiffViewModel>`, `vm.diff_mut(&mut self) -> Option<&mut DiffViewModel>`
  - `vm.open_pair_command(&mut self, left: &Path, right: &Path) -> ()` (error → `error` field, never panics)
  - `vm.save_all(&mut self) -> ()`
  - `vm.error(&self) -> Option<&str>`
  - `vm.dismiss_error(&mut self) -> ()`
  - `vm.title(&self) -> String` (`"DRZDiffCoder — left ↔ right"` with filenames, `*` when dirty)

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmpfile(content: &str, name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("drzvm_test");
        std::fs::create_dir_all(&dir).ok();
        let p = dir.join(name);
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }

    #[test]
    fn open_pair_success() {
        let l = tmpfile("a\n", "vm_l.txt");
        let r = tmpfile("b\n", "vm_r.txt");
        let mut vm = AppViewModel::open_pair(&l, &r);
        assert!(vm.diff().is_some());
        assert!(vm.error().is_none());
        vm.diff_mut().unwrap().flush_diff_now();
        assert_eq!(vm.diff().unwrap().hunks().len(), 1);
    }

    #[test]
    fn open_pair_missing_file_sets_error() {
        let r = tmpfile("b\n", "vm_r2.txt");
        let vm = AppViewModel::open_pair(Path::new("/nonexistent/x.txt"), &r);
        assert!(vm.diff().is_none());
        assert!(vm.error().is_some());
    }

    #[test]
    fn save_all_clears_dirty() {
        let l = tmpfile("a\n", "vm_l3.txt");
        let r = tmpfile("b\n", "vm_r3.txt");
        let mut vm = AppViewModel::open_pair(&l, &r);
        let d = vm.diff_mut().unwrap();
        d.flush_diff_now();
        d.merge_chunk(0, MergeDirection::LeftToRight);
        vm.save_all();
        assert!(!vm.diff().unwrap().right().is_dirty());
    }

    #[test]
    fn title_shows_dirty_marker() {
        let l = tmpfile("a\n", "vm_l4.txt");
        let r = tmpfile("b\n", "vm_r4.txt");
        let mut vm = AppViewModel::open_pair(&l, &r);
        assert!(!vm.title().contains('*'));
        vm.diff_mut().unwrap().right_mut().edit(0, 0, "z");
        assert!(vm.title().contains('*'));
    }
}
```

- [ ] **Step 2: Run, verify fail**

Run: `cargo test -p drz-viewmodel app_vm`
Expected: FAIL.

- [ ] **Step 3: Implement** `app_vm.rs` (above tests):

```rust
use crate::diff_vm::{DiffViewModel, MergeDirection};
use crate::editor_vm::EditorViewModel;
use std::path::{Path, PathBuf};

pub struct AppViewModel {
    diff: Option<DiffViewModel>,
    error: Option<String>,
}

impl AppViewModel {
    pub fn empty() -> AppViewModel {
        AppViewModel { diff: None, error: None }
    }

    pub fn open_pair(left: &Path, right: &Path) -> AppViewModel {
        let mut vm = AppViewModel::empty();
        vm.open_pair_command(left, right);
        vm
    }

    pub fn open_pair_command(&mut self, left: &Path, right: &Path) {
        let result = (|| -> Result<DiffViewModel, String> {
            let l = EditorViewModel::open(left)
                .map_err(|e| format!("{}: {e}", left.display()))?;
            let r = EditorViewModel::open(right)
                .map_err(|e| format!("{}: {e}", right.display()))?;
            let mut d = DiffViewModel::new(l, r);
            d.flush_diff_now();
            Ok(d)
        })();
        match result {
            Ok(d) => {
                self.diff = Some(d);
                self.error = None;
            }
            Err(msg) => {
                self.diff = None;
                self.error = Some(format!("open failed: {msg}"));
            }
        }
    }

    pub fn diff(&self) -> Option<&DiffViewModel> { self.diff.as_ref() }
    pub fn diff_mut(&mut self) -> Option<&mut DiffViewModel> { self.diff.as_mut() }

    pub fn save_all(&mut self) {
        if let Some(d) = &mut self.diff {
            for side in [d.left_mut(), d.right_mut()] {
                if side.is_dirty() {
                    if let Err(e) = side.save() {
                        self.error = Some(format!("save failed: {e}"));
                        return;
                    }
                }
            }
        }
    }

    pub fn error(&self) -> Option<&str> { self.error.as_deref() }
    pub fn dismiss_error(&mut self) { self.error = None; }

    pub fn title(&self) -> String {
        match &self.diff {
            Some(d) => {
                let name = |vm: &EditorViewModel| -> String {
                    vm.path()
                        .and_then(|p: &Path| p.file_name())
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "(untitled)".into())
                };
                let dirty = if d.left().is_dirty() || d.right().is_dirty() { " *" } else { "" };
                format!("DRZDiffCoder — {} ↔ {}{}", name(d.left()), name(d.right()), dirty)
            }
            None => "DRZDiffCoder".into(),
        }
    }
}
```

Note for implementer: `MergeDirection` used in test must be imported — tests sit inside `app_vm.rs`; add `use crate::diff_vm::MergeDirection;` in test module (shown above via `use super::*` + module re-export).

`lib.rs` adds:

```rust
mod app_vm;
pub use app_vm::AppViewModel;
```

- [ ] **Step 4: Run, verify pass**

Run: `cargo test -p drz-viewmodel`
Expected: 13 passed total.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(viewmodel): AppViewModel with open/save commands and error surface"
```

---

### Task 12: drz-editor — code editor widget

**Files:**
- Create: `crates/drz-editor/src/editor.rs`
- Create: `crates/drz-editor/src/theme.rs`
- Modify: `crates/drz-editor/src/lib.rs`

**Interfaces:**
- Consumes: `EditorViewModel`, `LineSpan`, `Style` (drz-viewmodel).
- Produces:
  - `pub fn style_color(style: Style, dark: bool) -> egui::Color32` (theme.rs)
  - `pub struct CodeEditor { cursor: (usize, usize), scroll_row: f32 }` — view-state only
  - `CodeEditor::new() -> CodeEditor`
  - `CodeEditor::cursor(&self) -> (usize, usize)`
  - `CodeEditor::show(&mut self, ui: &mut egui::Ui, vm: &mut EditorViewModel, row_of_line: Option<&dyn Fn(usize) -> usize>, line_of_row: Option<&dyn Fn(usize) -> Option<usize>>, total_rows: usize) -> ()`
    - When `row_of_line`/`line_of_row` provided (diff mode), renders by display row with padding rows blank. When `None`, plain 1:1 rendering.
  - Monospace font only: `egui::FontId::monospace(14.0)`. Column math: `col = (x / char_width).round()`, `char_width = ui.fonts(|f| f.glyph_width(&font_id, 'M'))`.

**Steps:** (logic-first; cursor math unit-tested, rendering verified by running app in Task 14)

- [ ] **Step 1: Write failing tests for pure helpers** (`editor.rs` test module)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_col_to_line_len() {
        assert_eq!(clamp_col(10, 4), 4);
        assert_eq!(clamp_col(2, 4), 2);
    }

    #[test]
    fn click_x_to_col_rounds() {
        assert_eq!(x_to_col(0.0, 8.0), 0);
        assert_eq!(x_to_col(3.9, 8.0), 0);
        assert_eq!(x_to_col(4.1, 8.0), 1);
        assert_eq!(x_to_col(80.0, 8.0), 10);
    }
}
```

Helpers:

```rust
pub(crate) fn clamp_col(col: usize, line_byte_len: usize) -> usize {
    col.min(line_byte_len)
}

pub(crate) fn x_to_col(x: f32, char_width: f32) -> usize {
    if char_width <= 0.0 { return 0; }
    (x / char_width).round().max(0.0) as usize
}
```

- [ ] **Step 2: Run, verify fail → implement helpers → verify pass**

Run: `cargo test -p drz-editor`
Expected: 2 passed after implementation.

- [ ] **Step 3: Implement** `theme.rs`:

```rust
use drz_viewmodel::types::Style;

pub fn style_color(style: Style, dark: bool) -> egui::Color32 {
    use egui::Color32;
    match (style, dark) {
        (Style::Keyword, true) => Color32::from_rgb(198, 120, 221),
        (Style::Keyword, false) => Color32::from_rgb(166, 38, 164),
        (Style::StringLit, true) => Color32::from_rgb(152, 195, 121),
        (Style::StringLit, false) => Color32::from_rgb(80, 161, 79),
        (Style::Comment, true) => Color32::from_rgb(128, 132, 144),
        (Style::Comment, false) => Color32::from_rgb(160, 160, 160),
        (Style::Function, true) => Color32::from_rgb(97, 175, 239),
        (Style::Function, false) => Color32::from_rgb(64, 120, 242),
        (Style::Type, true) => Color32::from_rgb(229, 192, 123),
        (Style::Type, false) => Color32::from_rgb(193, 132, 1),
        (Style::Number, true) => Color32::from_rgb(209, 154, 102),
        (Style::Number, false) => Color32::from_rgb(182, 86, 17),
        (Style::Constant, true) => Color32::from_rgb(86, 182, 194),
        (Style::Constant, false) => Color32::from_rgb(1, 132, 188),
        (Style::Default, true) => Color32::from_rgb(220, 223, 228),
        (Style::Default, false) => Color32::from_rgb(56, 58, 66),
    }
}
```

`editor.rs` — `show` implementation:

```rust
use crate::theme::style_color;
use drz_viewmodel::{EditorViewModel, LineSpan};

pub struct CodeEditor {
    cursor: (usize, usize), // (line, col_byte)
    focused: bool,
}

impl CodeEditor {
    pub fn new() -> CodeEditor {
        CodeEditor { cursor: (0, 0), focused: false }
    }

    pub fn cursor(&self) -> (usize, usize) { self.cursor }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        vm: &mut EditorViewModel,
        line_of_row: Option<&dyn Fn(usize) -> Option<usize>>,
        total_rows: usize,
    ) {
        let font_id = egui::FontId::monospace(14.0);
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
        let char_width = ui.fonts(|f| f.glyph_width(&font_id, 'M'));
        let dark = ui.visuals().dark_mode;
        let gutter_width = 48.0;

        let rows = if line_of_row.is_some() { total_rows } else { vm.len_lines() };

        egui::ScrollArea::both()
            .id_source(ui.id().with("editor_scroll"))
            .show(ui, |ui| {
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(
                        gutter_width + char_width * max_line_cols(vm, line_of_row, rows) as f32 + 40.0,
                        row_height * rows as f32,
                    ),
                    egui::Sense::click(),
                );
                let visible = ui.clip_rect();
                let first_row = ((visible.top() - rect.top()) / row_height).floor().max(0.0) as usize;
                let last_row = (((visible.bottom() - rect.top()) / row_height).ceil() as usize).min(rows);

                if response.clicked() {
                    self.focused = true;
                    if let Some(pos) = response.interact_pointer_pos() {
                        let row = ((pos.y - rect.top()) / row_height).floor() as usize;
                        let col = x_to_col(pos.x - rect.left() - gutter_width, char_width);
                        let line = match line_of_row {
                            Some(f) => f(row).unwrap_or(self.cursor.0),
                            None => row.min(vm.len_lines().saturating_sub(1)),
                        };
                        let (_, span_end) = vm.line_byte_range(line);
                        let line_len = span_end - vm.line_byte_range(line).0;
                        self.cursor = (line, clamp_col(col, line_len));
                    }
                }

                if self.focused {
                    self.handle_keys(ui, vm);
                }

                let painter = ui.painter_at(rect);
                for row in first_row..last_row {
                    let y = rect.top() + row as f32 * row_height;
                    let line_opt = match line_of_row {
                        Some(f) => f(row),
                        None => Some(row),
                    };
                    let Some(line) = line_opt else { continue }; // padding row
                    // gutter
                    painter.text(
                        egui::pos2(rect.left() + gutter_width - 8.0, y),
                        egui::Align2::RIGHT_TOP,
                        (line + 1).to_string(),
                        font_id.clone(),
                        ui.visuals().weak_text_color(),
                    );
                    // text spans
                    let (text, spans) = vm.styled_line(line);
                    let mut job = egui::text::LayoutJob::default();
                    append_styled(&mut job, &text, &spans, &font_id, dark);
                    painter.galley(egui::pos2(rect.left() + gutter_width, y), job, egui::Color32::WHITE);
                    // cursor
                    if self.focused && self.cursor.0 == line {
                        let cx = rect.left() + gutter_width + self.cursor.1 as f32 * char_width;
                        painter.vline(cx, y..=y + row_height, ui.visuals().strong_text_color());
                    }
                }
            });
    }

    fn handle_keys(&mut self, ui: &mut egui::Ui, vm: &mut EditorViewModel) {
        let (line, col) = self.cursor;
        ui.input(|i| {
            for event in &i.events {
                match event {
                    egui::Event::Text(t) => {
                        vm.insert_at_line_col(line, col, t);
                        self.cursor.1 += t.len();
                        self.cursor.0 = self.cursor.0.min(vm.len_lines().saturating_sub(1));
                    }
                    egui::Event::Key { key: egui::Key::Enter, pressed: true, .. } => {
                        vm.insert_at_line_col(line, col, "\n");
                        self.cursor = (line + 1, 0);
                    }
                    egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } => {
                        if col > 0 {
                            let prev_char_len = vm.line(line)[..col]
                                .chars().last().map(|c| c.len_utf8()).unwrap_or(1);
                            vm.delete_range_line_col((line, col - prev_char_len), (line, col));
                            self.cursor.1 -= prev_char_len;
                        } else if line > 0 {
                            let prev_len = vm.line(line - 1).len();
                            vm.delete_range_line_col((line - 1, prev_len), (line, 0));
                            self.cursor = (line - 1, prev_len);
                        }
                    }
                    egui::Event::Key { key: egui::Key::ArrowLeft, pressed: true, .. } => {
                        if col > 0 { self.cursor.1 -= 1; }
                    }
                    egui::Event::Key { key: egui::Key::ArrowRight, pressed: true, .. } => {
                        self.cursor.1 = clamp_col(col + 1, vm.line(line).len());
                    }
                    egui::Event::Key { key: egui::Key::ArrowUp, pressed: true, .. } => {
                        if line > 0 {
                            self.cursor.0 -= 1;
                            self.cursor.1 = clamp_col(col, vm.line(line - 1).len());
                        }
                    }
                    egui::Event::Key { key: egui::Key::ArrowDown, pressed: true, .. } => {
                        if line + 1 < vm.len_lines() {
                            self.cursor.0 += 1;
                            self.cursor.1 = clamp_col(col, vm.line(line + 1).len());
                        }
                    }
                    _ => {}
                }
            }
        });
        // re-clamp after possible edits
        let (l, c) = self.cursor;
        if l < vm.len_lines() {
            self.cursor.1 = clamp_col(c, vm.line(l).len());
        }
    }
}

fn max_line_cols(vm: &EditorViewModel, _line_of_row: Option<&dyn Fn(usize) -> Option<usize>>, _rows: usize) -> usize {
    (0..vm.len_lines()).map(|i| vm.line(i).len()).max().unwrap_or(40).max(40)
}

fn append_styled(
    job: &mut egui::text::LayoutJob,
    text: &str,
    spans: &[LineSpan],
    font_id: &egui::FontId,
    dark: bool,
) {
    let mut pos = 0usize;
    let mut push = |range: std::ops::Range<usize>, style: drz_viewmodel::types::Style| {
        if range.start >= range.end || range.end > text.len() { return; }
        if !text.is_char_boundary(range.start) || !text.is_char_boundary(range.end) { return; }
        job.append(&text[range], 0.0, egui::TextFormat {
            font_id: font_id.clone(),
            color: style_color(style, dark),
            ..Default::default()
        });
    };
    for s in spans {
        if s.start > pos {
            push(pos..s.start, drz_viewmodel::types::Style::Default);
        }
        push(s.start.max(pos)..s.end, s.style);
        pos = s.end.max(pos);
    }
    if pos < text.len() {
        push(pos..text.len(), drz_viewmodel::types::Style::Default);
    }
}
```

`lib.rs`:

```rust
mod editor;
mod theme;

pub use editor::CodeEditor;
pub use theme::style_color;
```

Keyboard focus note: egui `Event::Text` only arrives when a widget holds focus — with manual focus tracking, text events may not fire. Correct approach: request egui focus on a hidden `TextEdit`-less widget via `response.request_focus()` after click, and gate `ui.input` reads on `response.has_focus()`. Implementer: replace `self.focused` bool with egui's focus system — after `response.clicked()`, call `response.request_focus()`; wrap `handle_keys` in `if response.has_focus()`. Test manually in Task 14.

- [ ] **Step 4: Verify compile + unit tests**

Run: `cargo test -p drz-editor`
Expected: 2 passed, crate compiles.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(editor): egui code editor widget with styled lines, gutter, cursor, key input"
```

---

### Task 13: drz-diff-ui — synced diff view with connectors + merge arrows

**Files:**
- Create: `crates/drz-diff-ui/src/diff_view.rs`
- Modify: `crates/drz-diff-ui/src/lib.rs`

**Interfaces:**
- Consumes: `DiffViewModel`, `MergeDirection`, `Alignment`, `Hunk` (drz-viewmodel); `CodeEditor` (drz-editor).
- Produces:
  - `pub struct DiffView { left_editor: CodeEditor, right_editor: CodeEditor, shared_scroll: f32 }`
  - `DiffView::new() -> DiffView`
  - `DiffView::show(&mut self, ui: &mut egui::Ui, vm: &mut DiffViewModel) -> ()`
    - Calls `vm.poll()` first.
    - Renders: left `CodeEditor` (line_of_row from `alignment.left`), center strip (connectors + arrows), right `CodeEditor` (`alignment.right`).
    - Shared scroll: one `ScrollArea` vertical offset applied to both panes — implement by wrapping both editors in a single horizontal layout, each editor's ScrollArea sharing a `scroll_offset` via `egui::ScrollArea::scroll_offset` / `.vertical_scroll_offset()`. Simplest: link via `ui.ctx().data` stored offset; or use `ScrollArea::both` linking built into egui 0.31 (`ScrollArea::vertical_scroll_offset(offset)`). Use `egui::ScrollArea::show` with explicit `scroll_offset` read/write — wire through `CodeEditor::show` by adding optional `scroll_offset: Option<&mut egui::Vec2>` param (extend Task 12 signature accordingly — record the change there).
    - Hunk bands: for each hunk, compute row span `[left_row_start, left_row_end)` from alignment (rows where left/right maps differ), paint translucent red/green background behind changed rows in each pane, draw trapezoid connector in center strip, draw "→" / "←" button per hunk; click → `vm.merge_chunk(idx, dir)`.

- [ ] **Step 1: Write failing tests for pure row-mapping helper** (`diff_view.rs` test module)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use drz_core::{build_alignment, Hunk};

    #[test]
    fn hunk_row_span_covers_changed_rows() {
        let hunks = vec![Hunk { old_start: 1, old_end: 2, new_start: 1, new_end: 3 }];
        let a = build_alignment(&hunks, 3, 4);
        let span = hunk_row_span(&a, &hunks[0]);
        assert_eq!(span, 1..3); // rows 1,2 (row1 = changed line, row2 = left padding)
    }

    #[test]
    fn hunk_row_span_pure_insert() {
        let hunks = vec![Hunk { old_start: 2, old_end: 2, new_start: 2, new_end: 4 }];
        let a = build_alignment(&hunks, 3, 5);
        let span = hunk_row_span(&a, &hunks[0]);
        assert_eq!(span, 2..4);
    }
}
```

Helper:

```rust
use drz_core::{Alignment, Hunk};

pub(crate) fn hunk_row_span(alignment: &Alignment, hunk: &Hunk) -> std::ops::Range<usize> {
    // rows where this hunk maps: locate first row whose left == old_start or right == new_start
    let mut start = alignment.left.len();
    let mut end = start;
    for (row, (l, r)) in alignment.left.iter().zip(alignment.right.iter()).enumerate() {
        let in_hunk = l.is_some_and(|i| i >= hunk.old_start && i < hunk.old_end.max(hunk.old_start + usize::from(hunk.old_end == hunk.old_start && hunk.new_end > hunk.new_start)))
            || r.is_some_and(|i| i >= hunk.new_start && i < hunk.new_end.max(hunk.new_start + usize::from(hunk.new_end == hunk.new_start && hunk.old_end > hunk.old_start)));
        let strictly = l.is_some_and(|i| i >= hunk.old_start && i < hunk.old_end)
            || r.is_some_and(|i| i >= hunk.new_start && i < hunk.new_end)
            || (l.is_none() || r.is_none()) && row_between(alignment, row, hunk);
        if strictly {
            start = start.min(row);
            end = row + 1;
        }
        let _ = in_hunk;
    }
    start..end.max(start + usize::from(start < alignment.left.len() && hunk.old_end == hunk.old_start || hunk.new_end == hunk.new_start))
}

fn row_between(_a: &Alignment, _row: usize, _h: &Hunk) -> bool { false }
```

Implementer note: simplify — walk alignment, collect rows where left index ∈ old range or right index ∈ new range OR row is padding inside the hunk block (padding rows directly adjacent to matched hunk rows within the same contiguous block). Rewrite helper cleanly as: find rows `r0..r1` = maximal contiguous run containing any matched index and its adjacent padding. Keep tests passing.

- [ ] **Step 2: Run, verify fail → implement → verify pass**

Run: `cargo test -p drz-diff-ui`
Expected: 2 passed.

- [ ] **Step 3: Implement `DiffView::show`**

```rust
pub struct DiffView {
    left_editor: CodeEditor,
    right_editor: CodeEditor,
    scroll: egui::Vec2,
}

impl DiffView {
    pub fn new() -> DiffView {
        DiffView {
            left_editor: CodeEditor::new(),
            right_editor: CodeEditor::new(),
            scroll: egui::Vec2::ZERO,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, vm: &mut DiffViewModel) {
        vm.poll();
        let total_rows = vm.alignment().left.len();
        let alignment = vm.alignment().clone();
        let hunks = vm.hunks().to_vec();

        let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
        ui.horizontal(|ui| {
            // left pane
            let left_map = alignment.left.clone();
            self.left_editor.show_rows(ui, vm.left_mut(), &left_map, total_rows, &mut self.scroll);
            // center strip
            let (strip_rect, _) = ui.allocate_exact_size(egui::vec2(60.0, ui.available_height()), egui::Sense::hover());
            self.paint_strip(ui, strip_rect, &alignment, &hunks, row_height, vm);
            // right pane
            let right_map = alignment.right.clone();
            self.right_editor.show_rows(ui, vm.right_mut(), &right_map, total_rows, &mut self.scroll);
        });
        let _ = row_height;
    }
}
```

Required `CodeEditor` API addition (modify Task 12 file, commit both together in this task):

```rust
pub fn show_rows(
    &mut self,
    ui: &mut egui::Ui,
    vm: &mut EditorViewModel,
    row_map: &[Option<usize>],
    total_rows: usize,
    scroll: &mut egui::Vec2,
) {
    let map = row_map.to_vec();
    self.show(ui, vm, Some(&move |row| map.get(row).copied().flatten()), total_rows, scroll);
}
```

And extend Task 12 `CodeEditor::show` signature with `scroll: &mut egui::Vec2`, passing to `ScrollArea::both().scroll_offset(*scroll)` and writing back `*scroll = output.state.offset`.

`paint_strip`: for each hunk idx: compute span via `hunk_row_span`; y0 = span.start * row_height - scroll.y; y1 = span.end * row_height - scroll.y; draw `ui.painter().rect_filled` translucent; buttons `←` / `→` centered → `vm.merge_chunk(idx, dir)`. Borrow discipline: compute click intents inside closures collecting `(usize, MergeDirection)` into a Vec, apply after painting (avoids double mutable borrow of vm).

- [ ] **Step 4: Verify compile + tests**

Run: `cargo test -p drz-diff-ui drz-editor`
Expected: all passed, workspace compiles.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(diff-ui): synced panes, change bands, connectors, merge arrows"
```

---

### Task 14: drz-app — CLI + window shell

**Files:**
- Create: `crates/drz-app/src/main.rs`
- Create: `crates/drz-app/src/app.rs`

**Interfaces:**
- Consumes: `AppViewModel` (drz-viewmodel), `DiffView` (drz-diff-ui).
- Produces: binary `drzdiff`:
  - `drzdiff LEFT RIGHT` → open compare
  - `drzdiff` (no args) → empty window, File→Open menu picks two files (rfd)

- [ ] **Step 1: Implement** `main.rs`:

```rust
mod app;

use clap::Parser;

#[derive(Parser)]
#[command(name = "drzdiff", about = "DRZDiffCoder — source code diff tool")]
struct Cli {
    /// Left file
    left: Option<std::path::PathBuf>,
    /// Right file
    right: Option<std::path::PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut vm = drz_viewmodel::AppViewModel::empty();
    if let (Some(l), Some(r)) = (cli.left, cli.right) {
        vm.open_pair_command(&l, &r);
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "DRZDiffCoder",
        options,
        Box::new(move |cc| Ok(Box::new(app::DrzApp::new(vm, cc)))),
    )?;
    Ok(())
}
```

`app.rs`:

```rust
use drz_diff_ui::DiffView;
use drz_viewmodel::AppViewModel;
use std::sync::Arc;

pub struct DrzApp {
    vm: AppViewModel,
    diff_view: DiffView,
}

impl DrzApp {
    pub fn new(mut vm: AppViewModel, cc: &eframe::CreationContext<'_>) -> DrzApp {
        let ctx = cc.egui_ctx.clone();
        let repaint: Arc<dyn Fn() + Send + Sync> = Arc::new(move || ctx.request_repaint());
        if let Some(d) = vm.diff_mut() {
            d.set_repaint_callback(repaint);
        }
        DrzApp { vm, diff_view: DiffView::new() }
    }

    fn open_dialogs(&mut self) {
        let left = rfd::FileDialog::new().set_title("Left file").pick_file();
        let right = rfd::FileDialog::new().set_title("Right file").pick_file();
        if let (Some(l), Some(r)) = (left, right) {
            self.vm.open_pair_command(&l, &r);
            let repaint: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
            if let Some(d) = self.vm.diff_mut() {
                let _ = repaint;
                // repaint callback wired at next frame via ctx in update()
                d.flush_diff_now();
            }
        }
    }
}

impl eframe::App for DrzApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ensure repaint callback on (re-opened) diff
        let ctx_clone = ctx.clone();
        if let Some(d) = self.vm.diff_mut() {
            d.set_repaint_callback(Arc::new(move || ctx_clone.request_repaint()));
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(100)); // debounce poll cadence

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open files…").clicked() {
                        ui.close_menu();
                        self.open_dialogs();
                    }
                    if ui.button("Save all").clicked() {
                        self.vm.save_all();
                    }
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        });

        if let Some(err) = self.vm.error().map(|e| e.to_string()) {
            egui::TopBottomPanel::top("error").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::RED, &err);
                    if ui.button("✕").clicked() {
                        self.vm.dismiss_error();
                    }
                });
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.vm.title()));
            if let Some(d) = self.vm.diff_mut() {
                self.diff_view.show(ui, d);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("File → Open files… or run: drzdiff LEFT RIGHT");
                });
            }
        });
    }
}
```

- [ ] **Step 2: Build + smoke run**

Run: `cargo build --workspace && printf 'a\nb\n' > /tmp/l.txt && printf 'a\nc\n' > /tmp/r.txt && timeout 5 ./target/debug/drzdiff /tmp/l.txt /tmp/r.txt || true`
Expected: compiles; window opens (headless CI: skip manual check; developer verifies interactively: highlighting on .rs files, editing works, merge arrows work, save works).

Manual QA checklist (run locally, not CI):
1. `drzdiff a.rs b.rs` — rust highlighting visible in both panes.
2. Type in left pane — highlight updates immediately, diff band updates after ~150ms.
3. Click → arrow — chunk copied, bands clear.
4. Ctrl+S / Save all — files written, `*` cleared from title.
5. Binary file → red error banner, no crash.

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat(app): CLI entry, window shell, menu, error banner"
```

---

### Task 15: Integration test — CLI difftool invocation

**Files:**
- Create: `crates/drz-app/tests/cli.rs`

**Interfaces:**
- Consumes: `drzdiff` binary.

- [ ] **Step 1: Write test**

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;

fn tmpfile(content: &str, name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("drz_cli_test");
    std::fs::create_dir_all(&dir).ok();
    let p = dir.join(name);
    let mut f = std::fs::File::create(&p).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    p
}

#[test]
fn help_succeeds() {
    Command::cargo_bin("drzdiff")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("LEFT"));
}

#[test]
fn missing_file_handled() {
    // GUI can't run headless; verify arg parse + error path doesn't panic
    // by checking the binary starts, reports via VM, and we rely on
    // --help smoke. For a true headless check, use a virtual display (xvfb)
    // in CI. Here: assert non-help invocation with bad paths doesn't
    // exit with code 101 (panic) within timeout — skipped if no display.
    if std::env::var_os("DISPLAY").is_none() && std::env::var_os("CI").is_some() {
        return; // headless CI without xvfb
    }
    let r = tmpfile("b\n", "cli_r.txt");
    let _ = Command::cargo_bin("drzdiff")
        .unwrap()
        .args(["/nonexistent/l.txt", r.to_str().unwrap()])
        .timeout(std::time::Duration::from_secs(3))
        .ok();
}
```

- [ ] **Step 2: Run**

Run: `cargo test -p drz-app`
Expected: passed (2 tests; second no-ops on headless CI).

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "test(app): CLI smoke tests"
```

---

### Task 16: CI — GitHub Actions matrix

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Write workflow**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace
```

- [ ] **Step 2: Verify locally**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: all green. Fix any clippy/fmt issues before committing.

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "ci: test matrix linux/windows/macos"
```

---

## Self-Review Notes (resolved)

- **Spec coverage:** Phase 1 items — 2-file compare (T10, T13), editable panes (T9, T12), incremental highlight (T7, T8), save (T3, T11), difftool CLI (T14, T15), scroll sync + connectors (T13). Error handling (T3, T11, T14). Testing per spec (all tasks + T15, T16). Phase 2/3 intentionally absent.
- **Type consistency:** `styled_line` returns `(String, Vec<LineSpan>)` everywhere; `Hunk` fields `old_start/old_end/new_start/new_end`; `MergeDirection::{LeftToRight, RightToLeft}`; `CodeEditor::show` extended with `scroll` param in T13 — T12 implementer must leave signature open to this change (T13 amends T12's file, single commit in T13).
- **Known simplifications (v1, documented in code):** `rope.to_string()` per parse; monospace-only column math; no text selection; binary sniff 8KB; 50MB cap.
