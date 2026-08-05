use crate::theme::{inline_bg, line_bg, style_color};
use drz_viewmodel::{EditorViewModel, LineSpan};

/// Whole-row background tint category for a diff line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowBg {
    Added,
    Removed,
}

/// Per-display-row decoration: optional full-row background and intra-line
/// char-range emphasis. Built by the diff view from per-line status and
/// inline marks; passed to the editor via `show_rows`.
#[derive(Debug, Clone, Default)]
pub struct RowDecor {
    pub bg: Option<RowBg>,
    /// Char-index ranges (col-aligned in monospace) to emphasize within the row.
    pub inline: Vec<(usize, usize)>,
}

pub struct CodeEditor {
    cursor: (usize, usize), // (line, col_byte)
}

impl CodeEditor {
    pub fn new() -> CodeEditor {
        CodeEditor { cursor: (0, 0) }
    }

    pub fn cursor(&self) -> (usize, usize) {
        self.cursor
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        vm: &mut EditorViewModel,
        line_of_row: Option<&dyn Fn(usize) -> Option<usize>>,
        total_rows: usize,
        scroll: &mut egui::Vec2,
        row_decor: Option<&dyn Fn(usize) -> Option<RowDecor>>,
    ) {
        let font_id = egui::FontId::monospace(14.0);
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
        let char_width = ui.fonts(|f| f.glyph_width(&font_id, 'M'));
        let dark = ui.visuals().dark_mode;
        let gutter_width = 48.0;
        let pane_width = ui.available_width();

        let rows = if line_of_row.is_some() {
            total_rows
        } else {
            vm.len_lines()
        };

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
                let visible = ui.clip_rect();
                let first_row =
                    ((visible.top() - rect.top()) / row_height).floor().max(0.0) as usize;
                let last_row =
                    (((visible.bottom() - rect.top()) / row_height).ceil() as usize).min(rows);

                if response.clicked() {
                    response.request_focus();
                    if let Some(pos) = response.interact_pointer_pos() {
                        let row = ((pos.y - rect.top()) / row_height).floor() as usize;
                        let col = x_to_col(pos.x - rect.left() - gutter_width, char_width);
                        let line = match line_of_row {
                            Some(f) => f(row).unwrap_or(self.cursor.0),
                            None => row.min(vm.len_lines().saturating_sub(1)),
                        };
                        let (span_start, span_end) = vm.line_byte_range(line);
                        let line_len = span_end - span_start;
                        self.cursor = (line, clamp_col(col, line_len));
                    }
                }

                if response.has_focus() {
                    self.handle_keys(ui, vm);
                }

                let focused = response.has_focus();
                let painter = ui.painter_at(rect);
                // Paint row backgrounds + inline emphasis first, then text on top.
                for row in first_row..last_row {
                    let y = rect.top() + row as f32 * row_height;
                    if let Some(decor) = row_decor.and_then(|f| f(row)) {
                        if let Some(bg) = decor.bg {
                            painter.rect_filled(
                                egui::Rect::from_min_max(
                                    egui::pos2(rect.left(), y),
                                    egui::pos2(rect.left() + pane_width, y + row_height),
                                ),
                                0.0,
                                line_bg(bg, dark),
                            );
                            for &(s, e) in &decor.inline {
                                if s >= e {
                                    continue;
                                }
                                let (rx, rw) =
                                    inline_rect_x(s, e, rect.left() + gutter_width, char_width);
                                painter.rect_filled(
                                    egui::Rect::from_min_size(
                                        egui::pos2(rx, y),
                                        egui::vec2(rw, row_height),
                                    ),
                                    0.0,
                                    inline_bg(bg, dark),
                                );
                            }
                        }
                    }
                }
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
                    let galley = ui.fonts(|f| f.layout_job(job));
                    painter.galley(
                        egui::pos2(rect.left() + gutter_width, y),
                        galley,
                        egui::Color32::WHITE,
                    );
                    // cursor
                    if focused && self.cursor.0 == line {
                        let cx = rect.left() + gutter_width + self.cursor.1 as f32 * char_width;
                        painter.vline(
                            cx,
                            y..=y + row_height,
                            egui::Stroke::new(1.0, ui.visuals().strong_text_color()),
                        );
                    }
                }
            });
        *scroll = output.state.offset;
    }

    /// Row-aligned variant of [`CodeEditor::show`]: `row_map[row]` gives the
    /// document line for display row `row` (`None` = padding row), used by the
    /// side-by-side diff view to keep both panes row-for-row aligned.
    pub fn show_rows(
        &mut self,
        ui: &mut egui::Ui,
        vm: &mut EditorViewModel,
        row_map: &[Option<usize>],
        total_rows: usize,
        scroll: &mut egui::Vec2,
        decors: Option<&[RowDecor]>,
    ) {
        let decors_vec: Option<Vec<RowDecor>> = decors.map(|d| d.to_vec());
        let decor_fn: Option<Box<dyn Fn(usize) -> Option<RowDecor>>> = decors_vec.map(|d| {
            Box::new(move |row: usize| -> Option<RowDecor> { d.get(row).cloned() })
                as Box<dyn Fn(usize) -> Option<RowDecor>>
        });
        self.show(
            ui,
            vm,
            Some(&|row| row_map.get(row).copied().flatten()),
            total_rows,
            scroll,
            decor_fn.as_deref(),
        );
    }

    fn handle_keys(&mut self, ui: &mut egui::Ui, vm: &mut EditorViewModel) {
        let (line, col) = self.cursor;
        // Snap col to a char boundary before any slice/insert: arrows move
        // bytewise and can leave col mid-char. Covers Text/Enter (rope insert
        // requires boundary byte index) and Backspace (str slice).
        let col = if line < vm.len_lines() {
            floor_col_boundary(&vm.line(line), col)
        } else {
            col
        };
        self.cursor.1 = col;
        ui.input(|i| {
            for event in &i.events {
                match event {
                    egui::Event::Text(t) => {
                        vm.insert_at_line_col(line, col, t);
                        self.cursor.1 += t.len();
                        self.cursor.0 = self.cursor.0.min(vm.len_lines().saturating_sub(1));
                    }
                    egui::Event::Key {
                        key: egui::Key::Enter,
                        pressed: true,
                        ..
                    } => {
                        vm.insert_at_line_col(line, col, "\n");
                        self.cursor = (line + 1, 0);
                    }
                    egui::Event::Key {
                        key: egui::Key::Backspace,
                        pressed: true,
                        ..
                    } => {
                        if col > 0 {
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
                    } if col > 0 => {
                        self.cursor.1 -= 1;
                    }
                    egui::Event::Key {
                        key: egui::Key::ArrowRight,
                        pressed: true,
                        ..
                    } => {
                        self.cursor.1 = clamp_col(col + 1, vm.line(line).len());
                    }
                    egui::Event::Key {
                        key: egui::Key::ArrowUp,
                        pressed: true,
                        ..
                    } if line > 0 => {
                        self.cursor.0 -= 1;
                        self.cursor.1 = clamp_col(col, vm.line(line - 1).len());
                    }
                    egui::Event::Key {
                        key: egui::Key::ArrowDown,
                        pressed: true,
                        ..
                    } if line + 1 < vm.len_lines() => {
                        self.cursor.0 += 1;
                        self.cursor.1 = clamp_col(col, vm.line(line + 1).len());
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

impl Default for CodeEditor {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn clamp_col(col: usize, line_byte_len: usize) -> usize {
    col.min(line_byte_len)
}

pub(crate) fn x_to_col(x: f32, char_width: f32) -> usize {
    if char_width <= 0.0 {
        return 0;
    }
    (x / char_width).round().max(0.0) as usize
}

/// Pixel rectangle for an inline char range within a row.
/// Returns `(x, width)` relative to the pane's text area (after the gutter).
pub(crate) fn inline_rect_x(
    start_col: usize,
    end_col: usize,
    gutter_right_x: f32,
    char_width: f32,
) -> (f32, f32) {
    let x = gutter_right_x + start_col as f32 * char_width;
    let w = (end_col as f32 - start_col as f32) * char_width;
    (x, w)
}

/// Floor a byte col to the nearest char boundary (col semantics stay byte offsets).
/// Cursor col can land mid-char because arrows move bytewise; slicing/inserting
/// at a non-boundary byte would panic.
pub(crate) fn floor_col_boundary(line: &str, col: usize) -> usize {
    let mut col = col.min(line.len());
    while col > 0 && !line.is_char_boundary(col) {
        col -= 1;
    }
    col
}

fn max_line_cols(vm: &EditorViewModel) -> usize {
    (0..vm.len_lines())
        .map(|i| vm.line(i).len())
        .max()
        .unwrap_or(40)
        .max(40)
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
        if range.start >= range.end || range.end > text.len() {
            return;
        }
        if !text.is_char_boundary(range.start) || !text.is_char_boundary(range.end) {
            return;
        }
        job.append(
            &text[range],
            0.0,
            egui::TextFormat {
                font_id: font_id.clone(),
                color: style_color(style, dark),
                ..Default::default()
            },
        );
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

    #[test]
    fn inline_rect_x_math() {
        // gutter at 48, char_w 8
        assert_eq!(inline_rect_x(3, 7, 48.0, 8.0), (72.0, 32.0));
        assert_eq!(inline_rect_x(0, 5, 48.0, 8.0), (48.0, 40.0));
        assert_eq!(inline_rect_x(0, 0, 48.0, 8.0), (48.0, 0.0));
    }

    #[test]
    fn floor_col_boundary_snaps_to_char_start() {
        let s = "aé💣b"; // a=1B, é=2B, 💣=4B, b=1B
        assert_eq!(floor_col_boundary(s, 0), 0);
        assert_eq!(floor_col_boundary(s, 1), 1);
        assert_eq!(floor_col_boundary(s, 2), 1); // mid é
        assert_eq!(floor_col_boundary(s, 3), 3);
        assert_eq!(floor_col_boundary(s, 4), 3); // mid 💣
        assert_eq!(floor_col_boundary(s, 5), 3);
        assert_eq!(floor_col_boundary(s, 6), 3);
        assert_eq!(floor_col_boundary(s, 7), 7);
        assert_eq!(floor_col_boundary(s, 8), 8); // end of string
        assert_eq!(floor_col_boundary(s, 100), 8); // beyond len clamps
    }

    #[test]
    fn backspace_mid_char_col_no_panic() {
        // mirrors the Backspace code path: floor col, slice, measure prev char
        let line = "aé💣b";
        let col = floor_col_boundary(line, 5); // mid 💣
        let prev_char_len = line[..col]
            .chars()
            .last()
            .map(|c| c.len_utf8())
            .unwrap_or(1);
        assert_eq!((col, prev_char_len), (3, 2)); // deletes é, not partial 💣
    }
}
