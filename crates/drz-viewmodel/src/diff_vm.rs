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
            let alignment = build_alignment(&hunks, content_lines(&old), content_lines(&new));
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
        self.alignment = build_alignment(&self.hunks, content_lines(&old), content_lines(&new));
    }

    pub fn hunks(&self) -> &[Hunk] { &self.hunks }
    pub fn alignment(&self) -> &Alignment { &self.alignment }

    pub fn merge_chunk(&mut self, hunk_idx: usize, dir: MergeDirection) {
        let Some(h) = self.hunks.get(hunk_idx).copied() else { return };
        match dir {
            MergeDirection::LeftToRight => {
                let replacement = build_block(&self.left, h.old_start, h.old_end);
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

/// Content-line count matching `diff_lines` semantics: ropey's trailing
/// phantom line after a final '\n' is not a diff line.
fn content_lines(text: &str) -> usize {
    text.lines().count()
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
