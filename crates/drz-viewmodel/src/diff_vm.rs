use crate::editor_vm::EditorViewModel;
use drz_core::{build_alignment, diff_lines, Alignment, Hunk};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
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
    /// Set when a real edit is observed; the diff spawns once it has been
    /// stable for DEBOUNCE (burst debounce while typing).
    dirty_since: Option<Instant>,
    /// Edit sequences as of the last spawned/computed diff. A recompute is
    /// scheduled only when a side's `edit_seq` advances past this.
    last_diffed: (u64, u64),
    in_flight: bool,
    rx: Option<Receiver<(Vec<Hunk>, Alignment)>>,
    repaint: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl DiffViewModel {
    pub fn new(left: EditorViewModel, right: EditorViewModel) -> DiffViewModel {
        let last_diffed = (left.edit_seq(), right.edit_seq());
        DiffViewModel {
            left,
            right,
            hunks: Vec::new(),
            alignment: Alignment {
                left: Vec::new(),
                right: Vec::new(),
            },
            // prime one initial diff on the first poll
            dirty_since: Some(Instant::now() - DEBOUNCE),
            last_diffed,
            in_flight: false,
            rx: None,
            repaint: None,
        }
    }

    pub fn left(&self) -> &EditorViewModel {
        &self.left
    }
    pub fn left_mut(&mut self) -> &mut EditorViewModel {
        &mut self.left
    }
    pub fn right(&self) -> &EditorViewModel {
        &self.right
    }
    pub fn right_mut(&mut self) -> &mut EditorViewModel {
        &mut self.right
    }

    pub fn set_repaint_callback(&mut self, cb: Arc<dyn Fn() + Send + Sync>) {
        self.repaint = Some(cb);
    }

    /// Explicitly ask for a recompute after the debounce window.
    pub fn request_diff(&mut self) {
        self.dirty_since.get_or_insert(Instant::now());
    }

    /// Call at frame start. Returns true if hunks changed.
    pub fn poll(&mut self) -> bool {
        let mut updated = false;
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok((hunks, alignment)) => {
                    self.hunks = hunks;
                    self.alignment = alignment;
                    self.in_flight = false;
                    self.rx = None;
                    updated = true;
                    if let Some(cb) = &self.repaint {
                        cb();
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    // diff thread died (panic): don't stay in_flight forever,
                    // keep existing hunks
                    self.in_flight = false;
                    self.rx = None;
                }
            }
        }
        if self.left.edit_seq() != self.last_diffed.0 || self.right.edit_seq() != self.last_diffed.1
        {
            self.dirty_since.get_or_insert(Instant::now());
        }
        let ready = self.dirty_since.is_some_and(|t| t.elapsed() >= DEBOUNCE);
        if ready && !self.in_flight {
            self.spawn_diff();
        }
        updated
    }

    fn spawn_diff(&mut self) {
        self.dirty_since = None;
        self.in_flight = true;
        self.last_diffed = (self.left.edit_seq(), self.right.edit_seq());
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
        self.last_diffed = (self.left.edit_seq(), self.right.edit_seq());
        let old = self.left.document_text();
        let new = self.right.document_text();
        self.hunks = diff_lines(&old, &new);
        self.alignment = build_alignment(&self.hunks, content_lines(&old), content_lines(&new));
    }

    pub fn hunks(&self) -> &[Hunk] {
        &self.hunks
    }
    pub fn alignment(&self) -> &Alignment {
        &self.alignment
    }

    /// Test probe: a recompute is scheduled or running.
    #[cfg(test)]
    pub(crate) fn diff_pending(&self) -> bool {
        self.dirty_since.is_some() || self.in_flight
    }

    pub fn merge_chunk(&mut self, hunk_idx: usize, dir: MergeDirection) {
        let Some(h) = self.hunks.get(hunk_idx).copied() else {
            return;
        };
        match dir {
            MergeDirection::LeftToRight => {
                let replacement = build_block(&self.left, h.old_start, h.old_end);
                self.right
                    .replace_lines(h.new_start, h.new_end, &replacement);
            }
            MergeDirection::RightToLeft => {
                let replacement = build_block(&self.right, h.new_start, h.new_end);
                self.left
                    .replace_lines(h.old_start, h.old_end, &replacement);
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
    let mut s = (start..end)
        .map(|i| vm.line(i))
        .collect::<Vec<_>>()
        .join("\n");
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

    /// Merge the single hunk of a two-doc pair, then assert the merged side
    /// matches `expected`, both docs are byte-identical, and no hunks remain.
    fn assert_converges(left: &str, right: &str, dir: MergeDirection, expected: &str) {
        let mut vm = DiffViewModel::new(
            EditorViewModel::from_text(left, LanguageId::PlainText),
            EditorViewModel::from_text(right, LanguageId::PlainText),
        );
        vm.flush_diff_now();
        assert!(!vm.hunks().is_empty(), "precondition: pair must differ");
        vm.merge_chunk(0, dir);
        vm.flush_diff_now();
        let l = vm.left().document_text();
        let r = vm.right().document_text();
        let merged = match dir {
            MergeDirection::LeftToRight => &r,
            MergeDirection::RightToLeft => &l,
        };
        assert_eq!(merged, expected, "merged text mismatch ({dir:?})");
        assert_eq!(l, r, "documents not byte-identical after merge ({dir:?})");
        assert!(vm.hunks().is_empty(), "hunks stuck after merge ({dir:?})");
    }

    #[test]
    fn merge_converges_mid_doc_matrix() {
        // change
        assert_converges(
            "a\nb\nc\n",
            "a\nX\nc\n",
            MergeDirection::LeftToRight,
            "a\nb\nc\n",
        );
        assert_converges(
            "a\nb\nc\n",
            "a\nX\nc\n",
            MergeDirection::RightToLeft,
            "a\nX\nc\n",
        );
        // insert / delete (same pair, both directions)
        assert_converges("a\nc\n", "a\nb\nc\n", MergeDirection::LeftToRight, "a\nc\n");
        assert_converges(
            "a\nc\n",
            "a\nb\nc\n",
            MergeDirection::RightToLeft,
            "a\nb\nc\n",
        );
        assert_converges(
            "a\nb\nc\n",
            "a\nc\n",
            MergeDirection::LeftToRight,
            "a\nb\nc\n",
        );
        assert_converges("a\nb\nc\n", "a\nc\n", MergeDirection::RightToLeft, "a\nc\n");
    }

    #[test]
    fn merge_converges_delete_at_eof() {
        assert_converges("a\n", "a\nb\n", MergeDirection::LeftToRight, "a\n");
        assert_converges("a\n", "a\nb\n", MergeDirection::RightToLeft, "a\nb\n");
        assert_converges("a\nb\n", "a\n", MergeDirection::LeftToRight, "a\nb\n");
        assert_converges("a\nb\n", "a\n", MergeDirection::RightToLeft, "a\n");
    }

    #[test]
    fn merge_converges_trailing_newline_matrix() {
        assert_converges("a\nb", "a\nb\n", MergeDirection::LeftToRight, "a\nb");
        assert_converges("a\nb", "a\nb\n", MergeDirection::RightToLeft, "a\nb\n");
        assert_converges("a\nb\n", "a\nb", MergeDirection::LeftToRight, "a\nb\n");
        assert_converges("a\nb\n", "a\nb", MergeDirection::RightToLeft, "a\nb");
    }

    #[test]
    fn render_access_without_edits_schedules_no_recompute() {
        let mut vm = vm_pair();
        vm.flush_diff_now();
        // render-frame style access: mutable accessors, but no edits
        let _ = vm.left();
        let _ = vm.left_mut();
        let _ = vm.right();
        let _ = vm.right_mut();
        vm.poll();
        assert!(
            !vm.diff_pending(),
            "read-only access must not schedule a diff"
        );
        vm.poll();
        assert!(!vm.diff_pending());
        // an actual edit schedules exactly one recompute
        vm.right_mut().edit(2, 3, "b");
        vm.poll();
        assert!(vm.diff_pending(), "edit must schedule a diff");
    }

    #[test]
    fn poll_clears_in_flight_when_diff_thread_dies() {
        let mut vm = vm_pair();
        vm.flush_diff_now();
        // simulate a panicked diff thread: sender dropped, nothing sent
        let (tx, rx) = channel::<(Vec<Hunk>, Alignment)>();
        drop(tx);
        vm.rx = Some(rx);
        vm.in_flight = true;
        vm.poll();
        assert!(!vm.in_flight, "disconnected worker must clear in_flight");
        assert!(vm.rx.is_none());
    }
}
