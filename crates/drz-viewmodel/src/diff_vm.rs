use crate::editor_vm::EditorViewModel;
use crate::types::LineStatus;
use drz_core::{build_alignment, diff_lines, inline_diff_ranges, Alignment, Hunk};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Per-line change decoration derived from line hunks. Document-line indexed
/// (`text.lines().count()` length, matching `diff_lines` semantics).
#[derive(Debug, Default, Clone)]
pub struct LineDecor {
    pub status_left: Vec<LineStatus>,
    pub status_right: Vec<LineStatus>,
    pub inline_left: Vec<Option<Vec<(usize, usize)>>>,
    pub inline_right: Vec<Option<Vec<(usize, usize)>>>,
}

const DEBOUNCE: Duration = Duration::from_millis(150);
/// Per-pair line count above which inline char-diff is skipped (line bg still
/// applied). Keeps large hunk recomputation cheap.
const INLINE_HUNK_LINE_CAP: usize = 400;

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
    /// Per-line change decoration, document-line indexed.
    line_decor: LineDecor,
    /// Set when a real edit is observed; the diff spawns once it has been
    /// stable for DEBOUNCE (burst debounce while typing).
    dirty_since: Option<Instant>,
    /// Edit sequences as of the last spawned/computed diff. A recompute is
    /// scheduled only when a side's `edit_seq` advances past this.
    last_diffed: (u64, u64),
    in_flight: bool,
    rx: Option<Receiver<DiffResult>>,
    repaint: Option<Arc<dyn Fn() + Send + Sync>>,
}

struct DiffResult {
    hunks: Vec<Hunk>,
    alignment: Alignment,
    decor: LineDecor,
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
            line_decor: LineDecor::default(),
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
                Ok(result) => {
                    self.hunks = result.hunks;
                    self.alignment = result.alignment;
                    self.line_decor = result.decor;
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
            let decor = build_line_status(&old, &new, &hunks);
            let _ = tx.send(DiffResult {
                hunks,
                alignment,
                decor,
            });
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
        self.line_decor = build_line_status(&old, &new, &self.hunks);
    }

    pub fn hunks(&self) -> &[Hunk] {
        &self.hunks
    }
    pub fn alignment(&self) -> &Alignment {
        &self.alignment
    }
    pub fn line_status_left(&self) -> &[LineStatus] {
        &self.line_decor.status_left
    }
    pub fn line_status_right(&self) -> &[LineStatus] {
        &self.line_decor.status_right
    }
    pub fn inline_left(&self) -> &[Option<Vec<(usize, usize)>>] {
        &self.line_decor.inline_left
    }
    pub fn inline_right(&self) -> &[Option<Vec<(usize, usize)>>] {
        &self.line_decor.inline_right
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

/// Per-side line status + inline marks derived from line hunks and the full
/// document texts. Lines are content-line indexed (`text.lines().count()`,
/// matching `diff_lines` / `content_lines` semantics); `status_*` length =
/// `content_lines` of the respective side. `inline_*` length matches.
///
/// Pure insertions (`old_start == old_end`) and deletions (`new_start ==
/// new_end`) carry no paired lines → no inline marks on the receiving side.
fn build_line_status(old: &str, new: &str, hunks: &[Hunk]) -> LineDecor {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let mut status_left = vec![LineStatus::Unchanged; old_lines.len()];
    let mut status_right = vec![LineStatus::Unchanged; new_lines.len()];
    let mut inline_left: Vec<Option<Vec<(usize, usize)>>> = vec![None; old_lines.len()];
    let mut inline_right: Vec<Option<Vec<(usize, usize)>>> = vec![None; new_lines.len()];

    for h in hunks {
        // mark backgrounds
        for i in h.old_start..h.old_end {
            if let Some(s) = status_left.get_mut(i) {
                *s = LineStatus::Removed;
            }
        }
        for i in h.new_start..h.new_end {
            if let Some(s) = status_right.get_mut(i) {
                *s = LineStatus::Added;
            }
        }
        // inline marks only when both sides contribute lines
        let old_len = h.old_end - h.old_start;
        let new_len = h.new_end - h.new_start;
        if old_len == 0 || new_len == 0 {
            continue;
        }
        if old_len + new_len > INLINE_HUNK_LINE_CAP {
            continue;
        }
        let pairs = old_len.min(new_len);
        for k in 0..pairs {
            let lo = h.old_start + k;
            let ln = h.new_start + k;
            let (l, r) = inline_diff_ranges(old_lines[lo], new_lines[ln]);
            inline_left[lo] = Some(l);
            inline_right[ln] = Some(r);
        }
    }
    LineDecor {
        status_left,
        status_right,
        inline_left,
        inline_right,
    }
}

fn build_block(vm: &EditorViewModel, start: usize, end: usize) -> String {
    if start >= end {
        return String::new();
    }
    // clamp end to current line count so a stale hunk (e.g. a merge arrow
    // clicked before the background diff updated after an edit) can't panic
    // by indexing past the new EOF.
    let len = vm.len_lines();
    let end = end.min(len);
    if start >= end {
        return String::new();
    }
    let mut s = (start..end)
        .map(|i| vm.line(i))
        .collect::<Vec<_>>()
        .join("\n");
    // preserve trailing newline so following lines stay separate
    if end < len {
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::LineStatus::{Added, Removed, Unchanged};
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
        let (tx, rx) = channel::<DiffResult>();
        drop(tx);
        vm.rx = Some(rx);
        vm.in_flight = true;
        vm.poll();
        assert!(!vm.in_flight, "disconnected worker must clear in_flight");
        assert!(vm.rx.is_none());
    }

    fn statuses(vm: &DiffViewModel, side: Side) -> &[LineStatus] {
        match side {
            Side::Left => vm.line_status_left(),
            Side::Right => vm.line_status_right(),
        }
    }
    fn inlines(vm: &DiffViewModel, side: Side) -> &[Option<Vec<(usize, usize)>>] {
        match side {
            Side::Left => vm.inline_left(),
            Side::Right => vm.inline_right(),
        }
    }
    #[derive(Copy, Clone)]
    enum Side {
        Left,
        Right,
    }

    #[test]
    fn status_change_hunk_marks_added_removed_with_inline() {
        // "a\nb\nc\n" vs "a\nX\nc\n": hunk {old: 1..2, new: 1..2} (single-line replace)
        let mut vm = DiffViewModel::new(
            EditorViewModel::from_text("a\nb\nc\n", LanguageId::PlainText),
            EditorViewModel::from_text("a\nX\nc\n", LanguageId::PlainText),
        );
        vm.flush_diff_now();
        // content lines: left=3, right=3
        assert_eq!(statuses(&vm, Side::Left), &[Unchanged, Removed, Unchanged]);
        assert_eq!(statuses(&vm, Side::Right), &[Unchanged, Added, Unchanged]);
        // paired: 1 pair → inline present on both sides
        let l1 = inlines(&vm, Side::Left).get(1).unwrap();
        let r1 = inlines(&vm, Side::Right).get(1).unwrap();
        assert!(
            l1.is_some() && !l1.as_ref().unwrap().is_empty(),
            "left line 1 should have inline char ranges (b vs X differ)"
        );
        assert!(r1.is_some());
    }

    #[test]
    fn status_insert_only_no_inline() {
        // "a\nc\n" vs "a\nb\nc\n": hunk {old: 1..1, new: 1..2} (pure insert)
        let mut vm = DiffViewModel::new(
            EditorViewModel::from_text("a\nc\n", LanguageId::PlainText),
            EditorViewModel::from_text("a\nb\nc\n", LanguageId::PlainText),
        );
        vm.flush_diff_now();
        assert_eq!(statuses(&vm, Side::Left), &[Unchanged, Unchanged]);
        assert_eq!(statuses(&vm, Side::Right), &[Unchanged, Added, Unchanged]);
        // no paired line on the right (new_len=1, old_len=0 → 0 pairs)
        assert!(inlines(&vm, Side::Left).iter().all(|x| x.is_none()));
        assert!(inlines(&vm, Side::Right).iter().all(|x| x.is_none()));
    }

    #[test]
    fn status_delete_only_no_inline() {
        // "a\nb\nc\n" vs "a\nc\n": hunk {old: 1..2, new: 1..1}
        let mut vm = DiffViewModel::new(
            EditorViewModel::from_text("a\nb\nc\n", LanguageId::PlainText),
            EditorViewModel::from_text("a\nc\n", LanguageId::PlainText),
        );
        vm.flush_diff_now();
        assert_eq!(statuses(&vm, Side::Left), &[Unchanged, Removed, Unchanged]);
        assert_eq!(statuses(&vm, Side::Right), &[Unchanged, Unchanged]);
        assert!(inlines(&vm, Side::Left).iter().all(|x| x.is_none()));
        assert!(inlines(&vm, Side::Right).iter().all(|x| x.is_none()));
    }

    #[test]
    fn status_cap_skips_inline_but_keeps_bg() {
        // build a hunk with old_len+new_len > 400 lines: 250 + 250 = 500
        let left_lines: Vec<String> = (0..250).map(|i| format!("L{i}\n")).collect();
        let right_lines: Vec<String> = (0..250).map(|i| format!("R{i}\n")).collect();
        let left_text = left_lines.join("");
        let right_text = right_lines.join("");
        let mut vm = DiffViewModel::new(
            EditorViewModel::from_text(&left_text, LanguageId::PlainText),
            EditorViewModel::from_text(&right_text, LanguageId::PlainText),
        );
        vm.flush_diff_now();
        // there is exactly one big replace hunk {old: 0..250, new: 0..250}
        assert_eq!(vm.hunks().len(), 1);
        let h = &vm.hunks()[0];
        assert_eq!((h.old_end - h.old_start) + (h.new_end - h.new_start), 500);
        // backgrounds still applied
        assert!(statuses(&vm, Side::Left).contains(&Removed));
        assert!(statuses(&vm, Side::Right).contains(&Added));
        // but inline marks skipped due to cap
        for line in inlines(&vm, Side::Left)
            .iter()
            .chain(inlines(&vm, Side::Right))
        {
            assert!(
                line.is_none(),
                "inline must be skipped when hunk exceeds cap"
            );
        }
    }

    #[test]
    fn status_unaffected_outside_hunks() {
        let mut vm = DiffViewModel::new(
            EditorViewModel::from_text("a\nb\nc\nd\ne\n", LanguageId::PlainText),
            EditorViewModel::from_text("a\nX\nc\nd\nY\n", LanguageId::PlainText),
        );
        vm.flush_diff_now();
        // hunks: {old:1..2, new:1..2} and {old:4..5, new:4..5}
        // lines 0,2,3 untouched
        assert_eq!(
            statuses(&vm, Side::Left),
            &[Unchanged, Removed, Unchanged, Unchanged, Removed]
        );
        assert_eq!(
            statuses(&vm, Side::Right),
            &[Unchanged, Added, Unchanged, Unchanged, Added]
        );
    }

    /// Regression for the ropey panic from clicking the merge arrow while a
    /// background diff is still in flight (stale hunks, doc already shrunk).
    /// merge_chunk must clamp instead of panicking inside `Document::line`.
    #[test]
    fn merge_chunk_does_not_panic_on_stale_hunk() {
        let l = EditorViewModel::from_text("a\nb\nc\n", LanguageId::PlainText);
        let r = EditorViewModel::from_text("a\nb\nc\n", LanguageId::PlainText);
        let mut vm = DiffViewModel::new(l, r);
        // inject a stale hunk: pretend there is a diff at line 2..50
        // (way past the doc's 3 lines).
        vm.hunks = vec![Hunk {
            old_start: 2,
            old_end: 50,
            new_start: 2,
            new_end: 50,
        }];
        // must not panic
        vm.merge_chunk(0, MergeDirection::LeftToRight);
        // doc unchanged because the stale range was clamped to EOF
        assert_eq!(vm.left().document_text(), "a\nb\nc\n");
        assert_eq!(vm.right().document_text(), "a\nb\nc\n");
    }

    /// `Document::line` and `line_byte_range` must clamp instead of panicking
    /// when called with out-of-bounds indices (regression for the ropey panic).
    #[test]
    fn document_api_clamps_oob_indices() {
        let doc = drz_core::Document::from_text("a\nb\n");
        assert_eq!(doc.line(99), "");
        let eof = doc.to_string().len();
        assert_eq!(doc.line_byte_range(99), (eof, eof));
        // replace_lines past EOF is a no-op, not a panic
        let mut doc = drz_core::Document::from_text("a\nb\n");
        doc.replace_lines(99, 100, "X");
        assert_eq!(doc.to_string(), "a\nb\n");
    }
}
