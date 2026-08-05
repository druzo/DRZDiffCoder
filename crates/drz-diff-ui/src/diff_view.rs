use drz_editor::CodeEditor;
use drz_viewmodel::{Alignment, DiffViewModel, Hunk, MergeDirection};

const STRIP_WIDTH: f32 = 60.0;

/// Side-by-side synced diff view: two `CodeEditor` panes sharing one vertical
/// scroll offset, with a center strip showing per-hunk bands and merge arrows.
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
        let alignment = vm.alignment().clone();
        let hunks = vm.hunks().to_vec();
        let total_rows = alignment.left.len();

        let row_height = ui.text_style_height(&egui::TextStyle::Monospace);

        let full = ui.available_rect_before_wrap();
        let (left_rect, strip_rect, right_rect) = pane_rects(full, STRIP_WIDTH);

        // Reserve all three rects in the parent layout so the cursor advances.
        ui.allocate_rect(left_rect, egui::Sense::hover());
        ui.allocate_rect(strip_rect, egui::Sense::hover());
        ui.allocate_rect(right_rect, egui::Sense::hover());

        let scroll_y = self.scroll.y;
        paint_strip(ui, strip_rect, &alignment, &hunks, row_height, scroll_y, vm);

        let mut left_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(left_rect)
                .layout(egui::Layout::left_to_right(egui::Align::LEFT).with_main_wrap(false)),
        );
        self.left_editor.show_rows(
            &mut left_ui,
            vm.left_mut(),
            &alignment.left,
            total_rows,
            &mut self.scroll,
        );

        let mut right_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(right_rect)
                .layout(egui::Layout::left_to_right(egui::Align::LEFT).with_main_wrap(false)),
        );
        self.right_editor.show_rows(
            &mut right_ui,
            vm.right_mut(),
            &alignment.right,
            total_rows,
            &mut self.scroll,
        );
    }
}

/// Split `full` into three non-overlapping rects: left pane (half), strip
/// (`strip_w` wide), right pane (remaining half). Pane widths clamp to ≥0
/// when `full` is narrower than `strip_w`.
pub(crate) fn pane_rects(full: egui::Rect, strip_w: f32) -> (egui::Rect, egui::Rect, egui::Rect) {
    let strip_w = strip_w.min(full.width());
    let pane_w = ((full.width() - strip_w) / 2.0).max(0.0);
    let left = egui::Rect::from_min_size(full.min, egui::vec2(pane_w, full.height()));
    let strip = egui::Rect::from_min_size(
        egui::pos2(full.left() + pane_w, full.top()),
        egui::vec2(strip_w, full.height()),
    );
    let right = egui::Rect::from_min_size(
        egui::pos2(full.left() + pane_w + strip_w, full.top()),
        egui::vec2(pane_w, full.height()),
    );
    (left, strip, right)
}

impl Default for DiffView {
    fn default() -> Self {
        Self::new()
    }
}

/// Paint hunk bands + merge arrow buttons in the center strip.
/// Click intents are collected and applied after painting so no `vm` borrow
/// is held across egui closures.
fn paint_strip(
    ui: &mut egui::Ui,
    strip_rect: egui::Rect,
    alignment: &Alignment,
    hunks: &[Hunk],
    row_height: f32,
    scroll_y: f32,
    vm: &mut DiffViewModel,
) {
    let mut intents: Vec<(usize, MergeDirection)> = Vec::new();
    let painter = ui.painter_at(strip_rect);

    for (idx, hunk) in hunks.iter().enumerate() {
        let span = hunk_row_span(alignment, hunk);
        if span.start >= span.end {
            continue;
        }
        let y0 = strip_rect.top() + span.start as f32 * row_height - scroll_y;
        let y1 = strip_rect.top() + span.end as f32 * row_height - scroll_y;
        if y1 < strip_rect.top() || y0 > strip_rect.bottom() {
            continue; // band scrolled out of view
        }
        let band = egui::Rect::from_min_max(
            egui::pos2(strip_rect.left(), y0),
            egui::pos2(strip_rect.right(), y1),
        );
        painter.rect_filled(
            band,
            2.0,
            egui::Color32::from_rgba_unmultiplied(255, 196, 0, 40),
        );

        // merge buttons centered in the band (clamped into the strip)
        let mid_y = ((y0 + y1) / 2.0).clamp(strip_rect.top() + 8.0, strip_rect.bottom() - 8.0);
        let btn = egui::vec2(26.0, 16.0);
        let to_right =
            egui::Rect::from_center_size(egui::pos2(strip_rect.left() + 14.0, mid_y), btn);
        let to_left =
            egui::Rect::from_center_size(egui::pos2(strip_rect.right() - 14.0, mid_y), btn);
        if ui
            .put(to_right, egui::Button::new("\u{2192}").small())
            .on_hover_text("Apply left \u{2192} right")
            .clicked()
        {
            intents.push((idx, MergeDirection::LeftToRight));
        }
        if ui
            .put(to_left, egui::Button::new("\u{2190}").small())
            .on_hover_text("Apply right \u{2192} left")
            .clicked()
        {
            intents.push((idx, MergeDirection::RightToLeft));
        }
    }

    for (idx, dir) in intents {
        vm.merge_chunk(idx, dir);
    }
}

/// Display-row span covered by `hunk`: the maximal contiguous run of rows
/// where the left index is in `old_start..old_end`, the right index is in
/// `new_start..new_end`, or the row is a padding row (`None`) inside that
/// same contiguous block. Pure insertions (empty old range) match on the new
/// range plus adjacent left padding; pure deletions are symmetric.
pub(crate) fn hunk_row_span(alignment: &Alignment, hunk: &Hunk) -> std::ops::Range<usize> {
    let n = alignment.left.len().min(alignment.right.len());
    let matched = |row: usize| -> bool {
        alignment.left[row].is_some_and(|i| i >= hunk.old_start && i < hunk.old_end)
            || alignment.right[row].is_some_and(|i| i >= hunk.new_start && i < hunk.new_end)
    };
    let padding = |row: usize| alignment.left[row].is_none() || alignment.right[row].is_none();

    let mut row = 0;
    while row < n {
        if matched(row) || padding(row) {
            let start = row;
            let mut any_matched = false;
            while row < n && (matched(row) || padding(row)) {
                any_matched |= matched(row);
                row += 1;
            }
            if any_matched {
                return start..row;
            }
        } else {
            row += 1;
        }
    }
    // hunk matches no row (empty ranges): empty span
    0..0
}

#[cfg(test)]
mod tests {
    use super::*;
    use drz_viewmodel::{build_alignment, Hunk};

    #[test]
    fn hunk_row_span_covers_changed_rows() {
        let hunks = vec![Hunk {
            old_start: 1,
            old_end: 2,
            new_start: 1,
            new_end: 3,
        }];
        let a = build_alignment(&hunks, 3, 4);
        let span = hunk_row_span(&a, &hunks[0]);
        assert_eq!(span, 1..3); // rows 1,2 (row1 = changed line, row2 = left padding)
    }

    #[test]
    fn hunk_row_span_pure_insert() {
        let hunks = vec![Hunk {
            old_start: 2,
            old_end: 2,
            new_start: 2,
            new_end: 4,
        }];
        let a = build_alignment(&hunks, 3, 5);
        let span = hunk_row_span(&a, &hunks[0]);
        assert_eq!(span, 2..4);
    }

    fn rect(min: egui::Pos2, max: egui::Pos2) -> egui::Rect {
        egui::Rect { min, max }
    }

    #[test]
    fn pane_rects_half_width_full_height() {
        let full = rect(egui::pos2(0.0, 0.0), egui::pos2(1280.0, 800.0));
        let (l, s, r) = pane_rects(full, 60.0);
        // total = left + strip + right; strip centered between panes
        assert_eq!(l.width(), 610.0);
        assert_eq!(s.width(), 60.0);
        assert_eq!(r.width(), 610.0);
        assert_eq!(l.height(), 800.0);
        assert_eq!(s.height(), 800.0);
        assert_eq!(r.height(), 800.0);
        assert_eq!(l.left(), 0.0);
        assert_eq!(l.right(), 610.0);
        assert_eq!(s.left(), 610.0);
        assert_eq!(s.right(), 670.0);
        assert_eq!(r.left(), 670.0);
        assert_eq!(r.right(), 1280.0);
    }

    #[test]
    fn pane_rects_no_overlap() {
        let full = rect(egui::pos2(100.0, 50.0), egui::pos2(900.0, 550.0));
        let (l, s, r) = pane_rects(full, 40.0);
        assert!(l.right() <= s.left());
        assert!(s.right() <= r.left());
    }

    #[test]
    fn pane_rects_non_neg_width_when_strip_wider_than_full() {
        let full = rect(egui::pos2(0.0, 0.0), egui::pos2(50.0, 100.0));
        let (l, s, r) = pane_rects(full, 80.0);
        // strip shrinks to full width; panes clamp to zero width but valid rects
        assert!(l.width() >= 0.0);
        assert!(r.width() >= 0.0);
        assert_eq!(s.width(), 50.0);
        assert_eq!(s.left(), 0.0);
        assert_eq!(s.right(), 50.0);
        assert_eq!(l.left(), 0.0);
        assert_eq!(l.right(), 0.0);
        assert_eq!(r.left(), 50.0);
        assert_eq!(r.right(), 50.0);
    }
}
