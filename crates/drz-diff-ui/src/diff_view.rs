use drz_editor::{CodeEditor, RowBg, RowDecor};
use drz_viewmodel::{Alignment, DiffViewModel, Hunk, LineStatus, MergeDirection};

const STRIP_WIDTH: f32 = 60.0;

/// Merge-button icons embedded at compile time so the binary is self-contained
/// and works regardless of CWD / install layout.
const ARROW_RIGHT_PNG: &[u8] = include_bytes!("../../../icons/arrow.right.png");
const ARROW_LEFT_PNG: &[u8] = include_bytes!("../../../icons/arrow.left.png");

/// Side-by-side synced diff view: two `CodeEditor` panes sharing one vertical
/// scroll offset, with a center strip showing per-hunk bands and merge arrows.
pub struct DiffView {
    left_editor: CodeEditor,
    right_editor: CodeEditor,
    scroll: egui::Vec2,
    left_decors: Vec<RowDecor>,
    right_decors: Vec<RowDecor>,
    arrow_right: Option<egui::TextureHandle>,
    arrow_left: Option<egui::TextureHandle>,
}

impl DiffView {
    pub fn new() -> DiffView {
        DiffView {
            left_editor: CodeEditor::new(),
            right_editor: CodeEditor::new(),
            scroll: egui::Vec2::ZERO,
            left_decors: Vec::new(),
            right_decors: Vec::new(),
            arrow_right: None,
            arrow_left: None,
        }
    }

    /// Lazily decode the embedded PNGs into GPU textures on the first show().
    /// Errors fall back to the text glyph buttons (graceful degrade).
    fn ensure_textures(&mut self, ctx: &egui::Context) {
        if self.arrow_right.is_none() {
            self.arrow_right = load_png_texture(ctx, "drz_arrow_right", ARROW_RIGHT_PNG);
        }
        if self.arrow_left.is_none() {
            self.arrow_left = load_png_texture(ctx, "drz_arrow_left", ARROW_LEFT_PNG);
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, vm: &mut DiffViewModel) {
        vm.poll();
        self.ensure_textures(ui.ctx());
        let alignment = vm.alignment().clone();
        let hunks = vm.hunks().to_vec();
        let total_rows = alignment.left.len();

        let row_height = ui.text_style_height(&egui::TextStyle::Monospace);

        let full = ui.available_rect_before_wrap();
        // Reserve a header row above each pane — file name + line count.
        const HEADER_H: f32 = 28.0;
        let panes_full = egui::Rect::from_min_max(
            egui::pos2(full.left(), full.top() + HEADER_H),
            egui::pos2(full.right(), full.bottom()),
        );
        let (left_rect, strip_rect, right_rect) = pane_rects(panes_full, STRIP_WIDTH);

        // Pane headers.
        paint_pane_header(
            ui,
            egui::Rect::from_min_size(full.min, egui::vec2(left_rect.width(), HEADER_H)),
            vm.left().path(),
            vm.left().len_lines(),
            true,
        );
        paint_pane_header(
            ui,
            egui::Rect::from_min_max(
                egui::pos2(strip_rect.right(), full.top()),
                egui::pos2(right_rect.right(), full.top() + HEADER_H),
            ),
            vm.right().path(),
            vm.right().len_lines(),
            false,
        );

        // Reserve all three rects in the parent layout so the cursor advances.
        ui.allocate_rect(left_rect, egui::Sense::hover());
        ui.allocate_rect(strip_rect, egui::Sense::hover());
        ui.allocate_rect(right_rect, egui::Sense::hover());

        let scroll_y = self.scroll.y;
        self.paint_strip(ui, strip_rect, &alignment, &hunks, row_height, scroll_y, vm);

        // Build per-display-row decoration for each pane.
        build_decors(
            &alignment,
            vm.line_status_left(),
            vm.line_status_right(),
            vm.inline_left(),
            vm.inline_right(),
            &mut self.left_decors,
            &mut self.right_decors,
        );

        let mut left_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(left_rect)
                .id_salt("diff_left_pane")
                .layout(egui::Layout::left_to_right(egui::Align::LEFT).with_main_wrap(false)),
        );
        self.left_editor.show_rows(
            &mut left_ui,
            vm.left_mut(),
            &alignment.left,
            total_rows,
            &mut self.scroll,
            Some(&self.left_decors),
        );

        let mut right_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(right_rect)
                .id_salt("diff_right_pane")
                .layout(egui::Layout::left_to_right(egui::Align::LEFT).with_main_wrap(false)),
        );
        self.right_editor.show_rows(
            &mut right_ui,
            vm.right_mut(),
            &alignment.right,
            total_rows,
            &mut self.scroll,
            Some(&self.right_decors),
        );
    }
}

/// Paint a small header bar above each pane: colored side tag + file name +
/// line count. Subtle, doesn't compete with the code.
fn paint_pane_header(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    path: Option<&std::path::Path>,
    line_count: usize,
    is_left: bool,
) {
    let dark = ui.visuals().dark_mode;
    let bg = if dark {
        egui::Color32::from_rgb(20, 26, 50)
    } else {
        egui::Color32::from_rgb(245, 246, 250)
    };
    let accent = if is_left {
        egui::Color32::from_rgb(34, 211, 238)
    } else {
        egui::Color32::from_rgb(232, 121, 249)
    };
    let text = if dark {
        egui::Color32::from_rgb(220, 223, 232)
    } else {
        egui::Color32::from_rgb(28, 32, 48)
    };
    let dim = if dark {
        egui::Color32::from_rgb(148, 156, 178)
    } else {
        egui::Color32::from_rgb(96, 102, 120)
    };

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, bg);
    painter.rect_filled(
        egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 1.0)),
        0.0,
        if dark {
            egui::Color32::from_rgba_unmultiplied(80, 90, 130, 100)
        } else {
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 30)
        },
    );

    let tag = if is_left { "L" } else { "R" };
    let tag_pos = egui::pos2(rect.left() + 14.0, rect.top() + 6.0);
    painter.text(
        tag_pos,
        egui::Align2::LEFT_TOP,
        tag,
        egui::FontId::monospace(11.0),
        accent,
    );

    let name = path
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "(untitled)".into());
    let name_x = rect.left() + 30.0;
    painter.text(
        egui::pos2(name_x, rect.top() + 6.0),
        egui::Align2::LEFT_TOP,
        &name,
        egui::FontId::monospace(12.0),
        text,
    );

    let count_str = format!("{line_count} lines");
    let text_w = painter
        .layout(
            count_str.clone(),
            egui::FontId::proportional(11.0),
            dim,
            f32::INFINITY,
        )
        .size()
        .x;
    painter.text(
        egui::pos2(rect.right() - 14.0 - text_w, rect.top() + 8.0),
        egui::Align2::LEFT_TOP,
        count_str,
        egui::FontId::proportional(11.0),
        dim,
    );
}

/// Decode PNG bytes into an egui texture. Returns `None` on decode error so
/// callers can fall back gracefully.
fn load_png_texture(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<egui::TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = (img.width() as usize, img.height() as usize);
    let color = egui::ColorImage::from_rgba_unmultiplied([w, h], img.as_raw());
    Some(ctx.load_texture(name, color, egui::TextureOptions::LINEAR))
}

/// Map per-line status to a side-appropriate row background.
/// Left pane: Removed → Some(Removed), anything else → None.
/// Right pane: Added → Some(Added), anything else → None.
fn status_to_bg(st: LineStatus, side_is_left: bool) -> Option<RowBg> {
    match (st, side_is_left) {
        (LineStatus::Removed, true) => Some(RowBg::Removed),
        (LineStatus::Added, false) => Some(RowBg::Added),
        _ => None,
    }
}

/// Build display-row-indexed decoration vectors from alignment and per-side
/// line status / inline marks. Padding rows (`None` in alignment) get default
/// decoration (no bg, no inline).
fn build_decors(
    alignment: &Alignment,
    status_left: &[LineStatus],
    status_right: &[LineStatus],
    inline_left: &[Option<Vec<(usize, usize)>>],
    inline_right: &[Option<Vec<(usize, usize)>>],
    out_left: &mut Vec<RowDecor>,
    out_right: &mut Vec<RowDecor>,
) {
    out_left.clear();
    out_right.clear();
    let n = alignment.left.len().min(alignment.right.len());
    for row in 0..n {
        let l = alignment.left[row];
        let r = alignment.right[row];
        out_left.push(match l {
            Some(idx) => RowDecor {
                bg: status_left
                    .get(idx)
                    .copied()
                    .and_then(|s| status_to_bg(s, true)),
                inline: inline_left
                    .get(idx)
                    .and_then(|x| x.clone())
                    .unwrap_or_default(),
            },
            None => RowDecor::default(),
        });
        out_right.push(match r {
            Some(idx) => RowDecor {
                bg: status_right
                    .get(idx)
                    .copied()
                    .and_then(|s| status_to_bg(s, false)),
                inline: inline_right
                    .get(idx)
                    .and_then(|x| x.clone())
                    .unwrap_or_default(),
            },
            None => RowDecor::default(),
        });
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
impl DiffView {
    #[allow(clippy::too_many_arguments)]
    fn paint_strip(
        &self,
        ui: &mut egui::Ui,
        strip_rect: egui::Rect,
        alignment: &Alignment,
        hunks: &[Hunk],
        row_height: f32,
        scroll_y: f32,
        vm: &mut DiffViewModel,
    ) {
        let mut intents: Vec<(usize, MergeDirection)> = Vec::new();
        let dark = ui.visuals().dark_mode;
        let painter = ui.painter_at(strip_rect);

        let band_fill = egui::Color32::from_rgba_unmultiplied(
            34,
            211,
            238,
            if dark { 50 } else { 36 },
        );
        let band_edge = egui::Color32::from_rgb(34, 211, 238);

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
            painter.rect_filled(band, egui::CornerRadius::same(3), band_fill);
            // accent edge stripe on the left of the band
            painter.rect_filled(
                egui::Rect::from_min_size(
                    band.min,
                    egui::vec2(2.0, band.height()),
                ),
                egui::CornerRadius::same(1),
                band_edge,
            );

            // merge buttons centered in the band (clamped into the strip)
            let mid_y = ((y0 + y1) / 2.0).clamp(strip_rect.top() + 10.0, strip_rect.bottom() - 10.0);
            let btn = egui::vec2(22.0, 18.0);
            let to_right = egui::Rect::from_center_size(
                egui::pos2(strip_rect.center().x - 12.0, mid_y),
                btn,
            );
            let to_left = egui::Rect::from_center_size(
                egui::pos2(strip_rect.center().x + 12.0, mid_y),
                btn,
            );

            if ui
                .put(to_right, self.make_arrow_button(true))
                .on_hover_text("Apply left \u{2192} right")
                .clicked()
            {
                intents.push((idx, MergeDirection::LeftToRight));
            }
            if ui
                .put(to_left, self.make_arrow_button(false))
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

    /// Build a merge-arrow button using the embedded PNG icon (26x22 source
    /// rendered at 14x12 to fit the 22x18 button rect), falling back to the
    /// text arrow glyph if texture decoding failed.
    fn make_arrow_button(&self, right: bool) -> egui::Button<'static> {
        let tex = if right {
            self.arrow_right.as_ref()
        } else {
            self.arrow_left.as_ref()
        };
        match tex {
            Some(t) => egui::Button::image((t.id(), egui::vec2(14.0, 12.0)))
                .corner_radius(egui::CornerRadius::same(4))
                .min_size(egui::vec2(22.0, 18.0)),
            None => egui::Button::new(if right { "\u{2192}" } else { "\u{2190}" })
                .corner_radius(egui::CornerRadius::same(4))
                .min_size(egui::vec2(22.0, 18.0)),
        }
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

    fn dec_line(
        left: &[LineStatus],
        right: &[LineStatus],
        inline_l: &[Option<Vec<(usize, usize)>>],
        inline_r: &[Option<Vec<(usize, usize)>>],
        align: (Vec<Option<usize>>, Vec<Option<usize>>),
    ) -> (Vec<RowDecor>, Vec<RowDecor>) {
        let (al, ar) = align;
        let mut lo = Vec::new();
        let mut ro = Vec::new();
        build_decors(
            &Alignment {
                left: al,
                right: ar,
            },
            left,
            right,
            inline_l,
            inline_r,
            &mut lo,
            &mut ro,
        );
        (lo, ro)
    }

    #[test]
    fn status_to_bg_per_side() {
        assert_eq!(
            status_to_bg(LineStatus::Removed, true),
            Some(RowBg::Removed)
        );
        assert_eq!(
            status_to_bg(LineStatus::Added, true),
            None,
            "left pane ignores Added"
        );
        assert_eq!(status_to_bg(LineStatus::Added, false), Some(RowBg::Added));
        assert_eq!(
            status_to_bg(LineStatus::Removed, false),
            None,
            "right pane ignores Removed"
        );
        assert_eq!(status_to_bg(LineStatus::Unchanged, true), None);
        assert_eq!(status_to_bg(LineStatus::Unchanged, false), None);
    }

    #[test]
    fn build_decors_passes_inline_through() {
        // alignment row 0 → left line 0, right line 1
        let left = vec![LineStatus::Unchanged, LineStatus::Removed];
        let right = vec![LineStatus::Added, LineStatus::Unchanged];
        let il = vec![None, Some(vec![(2, 4)])];
        let ir = vec![Some(vec![(0, 1)]), None];
        let (lo, ro) = dec_line(
            &left,
            &right,
            &il,
            &ir,
            (vec![Some(0), Some(1)], vec![Some(1), Some(0)]),
        );
        // row 0: left[0]=Unchanged→no bg; right[1]=Unchanged→no bg; inline il[0]=None, ir[1]=None
        assert!(lo[0].bg.is_none() && ro[0].bg.is_none());
        assert!(lo[0].inline.is_empty() && ro[0].inline.is_empty());
        // row 1: left[1]=Removed→Some(Removed) + inline [(2,4)]; right[0]=Added→Some(Added) + inline [(0,1)]
        assert_eq!(lo[1].bg, Some(RowBg::Removed));
        assert_eq!(ro[1].bg, Some(RowBg::Added));
        assert_eq!(lo[1].inline, vec![(2, 4)]);
        assert_eq!(ro[1].inline, vec![(0, 1)]);
    }

    #[test]
    fn build_decors_padding_row_default() {
        // alignment: left None, right Some(0)
        let left = vec![LineStatus::Removed];
        let right = vec![LineStatus::Unchanged];
        let il = vec![Some(vec![(1, 2)])];
        let ir = vec![None];
        let (lo, ro) = dec_line(&left, &right, &il, &ir, (vec![None], vec![Some(0)]));
        assert!(lo[0].bg.is_none() && lo[0].inline.is_empty());
        assert!(ro[0].bg.is_none() && ro[0].inline.is_empty());
    }
}
