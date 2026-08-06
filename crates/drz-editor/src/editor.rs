use crate::theme::{inline_bg, line_bg, style_color};
use crate::EditorIcons;
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
    selection: Option<drz_viewmodel::Selection>,
    /// Anchor captured at drag start (left-button drag extends selection).
    /// `None` outside an active drag.
    drag_anchor: Option<(usize, usize)>,
    /// Timestamp + line of the most recent double-click, used to detect
    /// triple-click within 300 ms on the same line.
    last_double_click: Option<(std::time::Instant, usize)>,
    /// Last clipboard text captured from `egui::Event::Paste`. egui 0.31 has
    /// no synchronous clipboard read API, so the context menu uses this as
    /// the source for its Paste action. Populated whenever the user pastes
    /// while focus is in this editor; cleared when the menu consumes it.
    paste_text: Option<String>,
    /// Whether this editor's response held keyboard focus during the most
    /// recent `show()` call. Cached so the top-level Edit menu can route
    /// its actions to the focused pane without re-running the layout.
    focused: bool,
    icons: EditorIcons,
}

impl CodeEditor {
    pub fn new() -> CodeEditor {
        CodeEditor {
            cursor: (0, 0),
            selection: None,
            drag_anchor: None,
            last_double_click: None,
            paste_text: None,
            focused: false,
            icons: EditorIcons::new(),
        }
    }

    pub fn cursor(&self) -> (usize, usize) {
        self.cursor
    }

    /// Whether the editor currently owns keyboard focus. Updated by
    /// `show()` each frame.
    pub fn has_focus(&self) -> bool {
        self.focused
    }

    /// Pop the most recent edit. Restores `vm` to the previous state and
    /// moves the caret back to where the user was before the edit.
    /// No-op if the VM's undo stack is empty.
    pub fn undo(&mut self, vm: &mut EditorViewModel) {
        let current = self.cursor;
        if let Some(caret) = vm.undo(Some(current)) {
            self.cursor = caret;
            self.selection = None;
        }
    }

    /// Replay the most recently undone edit. No-op if the VM's redo stack
    /// is empty. The caret lands at the post-edit position the user last
    /// saw (tracked by the VM via the caret the editor passed to undo).
    pub fn redo(&mut self, vm: &mut EditorViewModel) {
        let current = self.cursor;
        if let Some(caret) = vm.redo(Some(current)) {
            self.cursor = caret;
        } else {
            // VM has no caret to restore (entry from a caret-less edit).
            // Clamp cursor to a valid position in the (possibly shortened)
            // document.
            if self.cursor.0 >= vm.len_lines() {
                self.cursor.0 = vm.len_lines().saturating_sub(1);
                self.cursor.1 = 0;
            } else {
                self.cursor.1 = self.cursor.1.min(vm.line(self.cursor.0).len());
            }
        }
        self.selection = None;
    }

    pub fn selection(&self) -> Option<&drz_viewmodel::Selection> {
        self.selection.as_ref()
    }

    pub fn set_selection(&mut self, sel: Option<drz_viewmodel::Selection>) {
        self.selection = sel;
    }

    pub fn set_cursor(&mut self, cursor: (usize, usize)) {
        self.cursor = cursor;
    }

    /// Take the most recently captured OS paste text. The editor stores it
    /// from `egui::Event::Paste` (egui 0.31 has no synchronous clipboard
    /// read API). Returns `None` if nothing was pasted yet.
    pub fn take_paste_text(&mut self) -> Option<String> {
        self.paste_text.take()
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        vm: &mut EditorViewModel,
        line_of_row: Option<&dyn Fn(usize) -> Option<usize>>,
        total_rows: usize,
        scroll: &mut egui::Vec2,
        row_decor: Option<&dyn Fn(usize) -> Option<RowDecor>>,
        scroll_target: &mut Option<f32>,
    ) {
        let font_id = egui::FontId::monospace(15.0);
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
        let char_width = ui.fonts(|f| f.glyph_width(&font_id, 'M'));
        let dark = ui.visuals().dark_mode;
        let gutter_width = 54.0;
        let pane_width = ui.available_width();
        let gutter_separator = egui::Stroke::new(
            1.0,
            if dark {
                egui::Color32::from_rgba_unmultiplied(80, 90, 130, 100)
            } else {
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 30)
            },
        );

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
                // Stable response ID: wrap the editor allocation in a
                // push_id so the Response.id is deterministic frame-to-frame
                // regardless of whether scroll bars are visible. Without
                // this, `next_auto_id_salt` drifts when the scroll bar
                // appears/disappears, breaking the focus-lock filter and
                // triggering egui's dead-mans-switch focus surrender.
                let inner = ui.push_id(egui::Id::new("editor_main"), |ui| {
                    ui.allocate_exact_size(
                        egui::vec2(
                            gutter_width + char_width * max_line_cols(vm) as f32 + 40.0,
                            row_height * rows as f32,
                        ),
                        egui::Sense::click_and_drag(),
                    )
                });
                let (rect, response) = inner.inner;
                let visible = ui.clip_rect();
                let first_row =
                    ((visible.top() - rect.top()) / row_height).floor().max(0.0) as usize;
                let last_row =
                    (((visible.bottom() - rect.top()) / row_height).ceil() as usize).min(rows);
                let visible_first = first_row;
                let visible_last = last_row;

                // Mouse interaction: click, drag, double-click, triple-click.
                self.icons.ensure_textures(ui.ctx());
                let mods = ui.ctx().input(|i| i.modifiers);
                let shift = mods.shift;

                if response.clicked() || response.drag_started() {
                    response.request_focus();
                }
                // Reassert the focus-lock filter every paint while focused.
                // The previous version only set this on click frames, which
                // let egui's spatial focus engine yank focus on Tab/arrows
                // after a resize or a side swap. The call is a no-op unless
                // the widget already has focus, so it's cheap.
                if response.has_focus() {
                    ui.ctx().memory_mut(|mem| {
                        mem.set_focus_lock_filter(
                            response.id,
                            egui::EventFilter {
                                tab: false,
                                horizontal_arrows: true,
                                vertical_arrows: true,
                                escape: false,
                            },
                        );
                    });
                }
                if let Some(pos) = response.interact_pointer_pos() {
                    let row = ((pos.y - rect.top()) / row_height).floor() as usize;
                    let col = x_to_col(pos.x - rect.left() - gutter_width, char_width);
                    // On a padding row the line_of_row closure returns
                    // None; clamp to the last valid line so a stale
                    // cursor (potentially == len_lines) can't survive
                    // into the next line_byte_range call and panic.
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
                            self.selection =
                                Some(drz_viewmodel::Selection::new(anchor, (line, clamped_col)));
                        }
                    } else if response.drag_stopped() {
                        self.drag_anchor = None;
                    } else if response.triple_clicked() {
                        // Triple-click: egui tags click 3 of a triple gesture
                        // as `triple_clicked` (NOT `double_clicked`), so this
                        // branch must come before the double-click arm or
                        // triple-click is unreachable.
                        response.request_focus();
                        self.apply_triple_click(line, vm);
                    } else if response.double_clicked() {
                        // Click 2 of a double-click (or click 3 of a triple
                        // whose third click egui already routed to the
                        // triple_clicked arm above). Snap-to-word pre-pass
                        // recovers from clicks that land on whitespace.
                        response.request_focus();
                        self.apply_double_click(line, clamped_col, vm);
                    } else if response.clicked_by(egui::PointerButton::Primary) {
                        // Primary (left) click only. Secondary clicks are
                        // handled by `response.context_menu(...)` below;
                        // letting them fall into this branch would collapse
                        // an existing selection on every right-click.
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
                        self.selection =
                            Some(drz_viewmodel::Selection::new(anchor, (line, clamped_col)));
                        if anchor == (line, clamped_col) {
                            self.selection = None;
                        }
                    }
                } else if response.clicked() {
                    // Click outside any visible row: collapse selection.
                    self.selection = None;
                    self.drag_anchor = None;
                }

                if response.has_focus() {
                    self.handle_keys(
                        ui,
                        vm,
                        scroll_target,
                        visible_first,
                        visible_last,
                        row_height,
                    );
                }

                // Right-click context menu.
                let has_sel = self.selection.is_some_and(|s| s.is_selected());
                let clipboard_has_text = self.paste_text.is_some();
                response.context_menu(|ui| {
                    let undo_label = if let Some(t) = self.icons.undo() {
                        egui::Button::image_and_text((t.id(), egui::vec2(14.0, 14.0)), "Undo")
                    } else {
                        egui::Button::new("Undo")
                    };
                    let redo_label = if let Some(t) = self.icons.redo() {
                        egui::Button::image_and_text((t.id(), egui::vec2(14.0, 14.0)), "Redo")
                    } else {
                        egui::Button::new("Redo")
                    };
                    if ui.add_enabled(vm.can_undo(), undo_label).clicked() {
                        self.undo(vm);
                        ui.close_menu();
                    }
                    if ui.add_enabled(vm.can_redo(), redo_label).clicked() {
                        self.redo(vm);
                        ui.close_menu();
                    }
                    ui.separator();
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
                        if let Some(text) = self.paste_text.as_ref() {
                            if !text.is_empty() {
                                let (s, e) = match self.selection {
                                    Some(sel) => sel.ordered(),
                                    None => (self.cursor, self.cursor),
                                };
                                let (nl, nc) = vm.replace_selection_with(s, e, text);
                                self.cursor = (nl, nc);
                                self.selection = None;
                            }
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    let select_all_label = if let Some(t) = self.icons.select_all() {
                        egui::Button::image_and_text((t.id(), egui::vec2(14.0, 14.0)), "Select All")
                    } else {
                        egui::Button::new("Select All")
                    };
                    if ui.add(select_all_label).clicked() {
                        let last = vm.len_lines().saturating_sub(1);
                        let last_len = if last < vm.len_lines() {
                            vm.line(last).len()
                        } else {
                            0
                        };
                        self.selection =
                            Some(drz_viewmodel::Selection::new((0, 0), (last, last_len)));
                        self.cursor = (last, last_len);
                        ui.close_menu();
                    }
                });

                let focused = response.has_focus();
                // Cache focus state for the top-level Edit menu to route
                // undo/redo/cut/copy/paste/select-all to the right pane.
                self.focused = focused;
                let painter = ui.painter_at(rect);
                // Gutter background (subtle, separates from the code area).
                let gutter_bg = if dark {
                    egui::Color32::from_rgb(20, 26, 50)
                } else {
                    egui::Color32::from_rgb(240, 242, 248)
                };
                painter.rect_filled(
                    egui::Rect::from_min_size(rect.min, egui::vec2(gutter_width, rect.height())),
                    0.0,
                    gutter_bg,
                );
                painter.vline(
                    rect.left() + gutter_width,
                    rect.top()..=rect.bottom(),
                    gutter_separator,
                );

                // Paint row backgrounds + inline emphasis first, then
                // selection highlight on top of those (so the selection tint
                // is visible against added/removed row backgrounds), and
                // finally text + cursor on top of everything.
                for row in first_row..last_row {
                    let y = rect.top() + row as f32 * row_height;
                    if let Some(decor) = row_decor.and_then(|f| f(row)) {
                        if let Some(bg) = decor.bg {
                            painter.rect_filled(
                                egui::Rect::from_min_max(
                                    egui::pos2(rect.left() + gutter_width, y),
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
                // Selection highlight pass: translucent overlay per row that
                // intersects the active selection. Ordered endpoints are
                // snapshotted up front so we don't fight the `&mut vm` borrow
                // inside the row loop.
                let sel_data: Option<((usize, usize), (usize, usize))> =
                    self.selection.as_ref().and_then(|s| {
                        if s.is_selected() {
                            Some(s.ordered())
                        } else {
                            None
                        }
                    });
                let sel_color = if dark {
                    egui::Color32::from_rgba_unmultiplied(120, 170, 255, 70)
                } else {
                    egui::Color32::from_rgba_unmultiplied(40, 90, 200, 40)
                };
                if let Some((sel_start, sel_end)) = sel_data {
                    for row in first_row..last_row {
                        let y = rect.top() + row as f32 * row_height;
                        let line_opt = match line_of_row {
                            Some(f) => f(row),
                            None => Some(row),
                        };
                        let Some(line) = line_opt else {
                            continue; // padding row
                        };
                        let (_ls, le) = vm.line_byte_range(line);
                        let line_len = le - _ls;
                        if let Some((cs, ce)) =
                            selection_per_line_range(line, line_len, sel_start, sel_end)
                        {
                            let x = rect.left() + gutter_width + cs as f32 * char_width;
                            let w = (ce - cs) as f32 * char_width;
                            painter.rect_filled(
                                egui::Rect::from_min_size(
                                    egui::pos2(x, y),
                                    egui::vec2(w, row_height),
                                ),
                                0.0,
                                sel_color,
                            );
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
                                                                 // gutter line number
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
                        ui.visuals().text_color(),
                    );
                    // cursor: 500ms half-period blink, only when focused and
                    // on this row. request_repaint keeps the animation
                    // ticking even when nothing else invalidates the frame.
                    if focused && self.cursor.0 == line {
                        let time = ui.ctx().input(|i| i.time);
                        let blink_on = (time % 1.0) < 0.5;
                        if blink_on {
                            let cx = rect.left() + gutter_width + self.cursor.1 as f32 * char_width;
                            painter.vline(
                                cx,
                                y..=y + row_height,
                                egui::Stroke::new(1.0, ui.visuals().strong_text_color()),
                            );
                        }
                    }
                }
                if focused {
                    ui.ctx().request_repaint();
                }
            });
        // handle_keys may have requested that the viewport scroll to a
        // specific pixel offset (PageUp / PageDown / auto-scroll). Apply
        // it before reading ScrollArea's final offset so the two panes
        // stay in sync. The DiffView passes a SHARED scroll_target so the
        // right editor sees the left editor's request — neither side can
        // silently overwrite the other.
        if let Some(target_y) = *scroll_target {
            *scroll = egui::vec2(scroll.x, target_y.max(0.0));
        } else {
            *scroll = output.state.offset;
        }
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
        scroll_target: &mut Option<f32>,
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
            scroll_target,
        );
    }

    fn handle_keys(
        &mut self,
        ui: &mut egui::Ui,
        vm: &mut EditorViewModel,
        scroll_target: &mut Option<f32>,
        visible_first: usize,
        visible_last: usize,
        row_height: f32,
    ) {
        use drz_viewmodel::Selection;

        // Clone events up front so we can mutate `self` between iterations.
        // The previous version captured `line`/`col` ONCE at the top and
        // reused stale values across multiple events in the same frame
        // (auto-repeat, key chords), which corrupted typing / backspace
        // state.
        let events: Vec<egui::Event> = ui.input(|i| i.events.clone());
        let mods = ui.ctx().input(|i| i.modifiers);
        let cmd_or_ctrl = mods.command;
        let shift = mods.shift;

        // Mutable cursor state, kept in sync with `self.cursor` after each
        // event. Per-event re-floor so multi-byte columns don't drift.
        let mut line = self.cursor.0;
        let mut col = if line < vm.len_lines() {
            floor_col_boundary(&vm.line(line), self.cursor.1)
        } else {
            self.cursor.1
        };
        self.cursor = (line, col);

        // Deferred OS-level actions: clipboard events fire from the OS layer
        // in any order and need to run after the event loop so we don't
        // borrow `vm` while inside the closure.
        let mut paste_text: Option<String> = None;
        let mut copy_request: Option<String> = None;
        let mut cut_request: Option<((usize, usize), (usize, usize))> = None;
        let mut select_all_request = false;
        let mut undo_request = false;
        let mut redo_request = false;

        for event in &events {
            match event {
                egui::Event::Text(t) => {
                    apply_text_insert(vm, t, &mut line, &mut col, &mut self.selection);
                    if line < vm.len_lines() {
                        col = floor_col_boundary(&vm.line(line), col);
                    }
                    self.cursor = (line, col);
                    ensure_line_visible(
                        scroll_target,
                        line,
                        row_height,
                        visible_first,
                        visible_last,
                    );
                }
                egui::Event::Paste(s) => {
                    paste_text = Some(s.clone());
                    self.paste_text = Some(s.clone());
                }
                egui::Event::Copy => {
                    copy_request = self.selection.and_then(|sel| {
                        let (s, e) = sel.ordered();
                        if s == e {
                            None
                        } else {
                            Some(vm.text_in_range(s, e))
                        }
                    });
                }
                egui::Event::Cut => {
                    cut_request = self.selection.and_then(|sel| {
                        let (s, e) = sel.ordered();
                        if s == e {
                            None
                        } else {
                            Some((s, e))
                        }
                    });
                }
                egui::Event::Key {
                    key: egui::Key::Z,
                    pressed: true,
                    ..
                } if cmd_or_ctrl => {
                    if shift {
                        redo_request = true;
                    } else {
                        undo_request = true;
                    }
                }
                egui::Event::Key {
                    key: egui::Key::Y,
                    pressed: true,
                    ..
                } if cmd_or_ctrl && !shift => {
                    redo_request = true;
                }
                egui::Event::Key {
                    key: egui::Key::A,
                    pressed: true,
                    ..
                } if cmd_or_ctrl => {
                    select_all_request = true;
                }
                egui::Event::Key {
                    key: egui::Key::Enter,
                    pressed: true,
                    ..
                } => {
                    apply_text_insert(vm, "\n", &mut line, &mut col, &mut self.selection);
                    line = (line + 1).min(vm.len_lines().saturating_sub(1));
                    col = 0;
                    self.cursor = (line, col);
                    ensure_line_visible(
                        scroll_target,
                        line,
                        row_height,
                        visible_first,
                        visible_last,
                    );
                }
                egui::Event::Key {
                    key: egui::Key::Backspace,
                    pressed: true,
                    ..
                } => {
                    apply_backspace(vm, &mut line, &mut col, &mut self.selection);
                    self.cursor = (line, col);
                    ensure_line_visible(
                        scroll_target,
                        line,
                        row_height,
                        visible_first,
                        visible_last,
                    );
                }
                egui::Event::Key {
                    key: egui::Key::Delete,
                    pressed: true,
                    ..
                } => {
                    apply_delete(vm, &mut line, &mut col, &mut self.selection);
                    self.cursor = (line, col);
                    ensure_line_visible(
                        scroll_target,
                        line,
                        row_height,
                        visible_first,
                        visible_last,
                    );
                }
                egui::Event::Key {
                    key: egui::Key::ArrowLeft,
                    pressed: true,
                    ..
                } => {
                    apply_arrow_left(shift, &mut line, &mut col, &mut self.selection);
                    self.cursor = (line, col);
                }
                egui::Event::Key {
                    key: egui::Key::ArrowRight,
                    pressed: true,
                    ..
                } => {
                    let line_len = if line < vm.len_lines() {
                        vm.line(line).len()
                    } else {
                        0
                    };
                    apply_arrow_right(shift, &mut line, &mut col, &mut self.selection, line_len);
                    self.cursor = (line, col);
                }
                egui::Event::Key {
                    key: egui::Key::ArrowUp,
                    pressed: true,
                    ..
                } => {
                    if line > 0 {
                        let prev_len = vm.line(line - 1).len();
                        apply_arrow_vert(
                            true,
                            shift,
                            &mut line,
                            &mut col,
                            &mut self.selection,
                            prev_len,
                        );
                        self.cursor = (line, col);
                        ensure_line_visible(
                            scroll_target,
                            line,
                            row_height,
                            visible_first,
                            visible_last,
                        );
                    }
                }
                egui::Event::Key {
                    key: egui::Key::ArrowDown,
                    pressed: true,
                    ..
                } => {
                    if line + 1 < vm.len_lines() {
                        let next_len = vm.line(line + 1).len();
                        apply_arrow_vert(
                            false,
                            shift,
                            &mut line,
                            &mut col,
                            &mut self.selection,
                            next_len,
                        );
                        self.cursor = (line, col);
                        ensure_line_visible(
                            scroll_target,
                            line,
                            row_height,
                            visible_first,
                            visible_last,
                        );
                    }
                }
                egui::Event::Key {
                    key: egui::Key::Home,
                    pressed: true,
                    ..
                } => {
                    apply_home(shift, &mut line, &mut col, &mut self.selection);
                    self.cursor = (line, col);
                }
                egui::Event::Key {
                    key: egui::Key::End,
                    pressed: true,
                    ..
                } => {
                    let line_len = if line < vm.len_lines() {
                        vm.line(line).len()
                    } else {
                        0
                    };
                    apply_end(shift, line_len, &mut line, &mut col, &mut self.selection);
                    self.cursor = (line, col);
                }
                egui::Event::Key {
                    key: egui::Key::PageUp,
                    pressed: true,
                    ..
                } => {
                    let visible_rows = visible_last.saturating_sub(visible_first).max(1);
                    let page = page_lines_for(visible_rows);
                    apply_page_up(vm, page, shift, &mut line, &mut col, &mut self.selection);
                    self.cursor = (line, col);
                    ensure_line_visible(
                        scroll_target,
                        line,
                        row_height,
                        visible_first,
                        visible_last,
                    );
                }
                egui::Event::Key {
                    key: egui::Key::PageDown,
                    pressed: true,
                    ..
                } => {
                    let visible_rows = visible_last.saturating_sub(visible_first).max(1);
                    let page = page_lines_for(visible_rows);
                    apply_page_down(vm, page, shift, &mut line, &mut col, &mut self.selection);
                    self.cursor = (line, col);
                    ensure_line_visible(
                        scroll_target,
                        line,
                        row_height,
                        visible_first,
                        visible_last,
                    );
                }
                _ => {}
            }
        }

        // Post-process deferred OS-level actions.
        if let Some(text) = copy_request {
            ui.ctx().copy_text(text);
        }
        if let Some((s, e)) = cut_request {
            let text = vm.text_in_range(s, e);
            ui.ctx().copy_text(text);
            let (nl, nc) = vm.replace_selection_with(s, e, "");
            self.cursor = (nl, nc);
            self.selection = None;
        }
        if let Some(text) = paste_text {
            if let Some(sel) = self.selection.take() {
                let (s, e) = sel.ordered();
                let (nl, nc) = vm.replace_selection_with(s, e, &text);
                self.cursor = (nl, nc);
            } else {
                let (nl, nc) = vm.replace_selection_with(self.cursor, self.cursor, &text);
                self.cursor = (nl, nc);
            }
            let (l, c) = self.cursor;
            if l < vm.len_lines() {
                self.cursor.1 = clamp_col(c, vm.line(l).len());
            }
        }
        if select_all_request {
            let last = vm.len_lines().saturating_sub(1);
            let last_len = if last < vm.len_lines() {
                vm.line(last).len()
            } else {
                0
            };
            self.selection = Some(Selection::new((0, 0), (last, last_len)));
            self.cursor = (last, last_len);
        }
        if undo_request {
            self.undo(vm);
        }
        if redo_request {
            self.redo(vm);
        }

        // Final cursor col clamp after the last event.
        let (l, c) = self.cursor;
        if l < vm.len_lines() {
            self.cursor.1 = clamp_col(c, vm.line(l).len());
        }
    }

    /// Double-click handler: select the word under `line` at `clicked_col`.
    /// Snap-to-word recovers from clicks on whitespace. Extracted as a method
    /// so it can be unit-tested without an `egui::Context`.
    fn apply_double_click(&mut self, line: usize, clicked_col: usize, vm: &EditorViewModel) {
        // `Selection` coordinates are (line, byte_col-into-line), not
        // (line, document-byte-offset). `word_range` already returns offsets
        // within `text_bytes`, so use them directly — do NOT add
        // `vm.line_byte_range(line).0` (the document start of the line) or
        // the coords become wrong for any line other than the first. (Caught
        // by manual smoke on develop: line-N selections rendered empty.)
        let text_bytes = vm.line(line);
        let target_col = snap_to_nearest_word(&text_bytes, clicked_col);
        let (l, r) = word_range(&text_bytes, target_col);
        self.cursor = (line, r);
        self.selection = Some(drz_viewmodel::Selection::new((line, l), (line, r)));
        self.drag_anchor = None;
        // Clear (do not record) the double-click timestamp. Triple-click is
        // now handled directly by `apply_triple_click` on `triple_clicked`
        // events, so a stale timestamp would be dead weight.
        self.last_double_click = None;
    }

    /// Triple-click handler: select the entire `line` (line content, no
    /// trailing newline). Extracted for unit-testability.
    fn apply_triple_click(&mut self, line: usize, vm: &EditorViewModel) {
        let line_len = vm.line(line).len();
        self.cursor = (line, line_len);
        self.selection = Some(drz_viewmodel::Selection::new((line, 0), (line, line_len)));
        self.drag_anchor = None;
        self.last_double_click = None;
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

/// Return the byte col of the nearest ASCII word char to `col` in `line`.
/// Used by double-click to recover from clicks that land on whitespace:
/// `word_range` returns empty on non-word bytes, so without this pre-pass a
/// click on the space between two words silently does nothing.
pub(crate) fn snap_to_nearest_word(line: &str, col: usize) -> usize {
    let bytes = line.as_bytes();
    let len = bytes.len();
    if len == 0 {
        return col;
    }
    let col = col.min(len);
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    if col < len && is_word(bytes[col]) {
        return col;
    }
    // Prefer the word on the right of the gap.
    let mut right = col + 1;
    while right < len && !is_word(bytes[right]) {
        right += 1;
    }
    if right < len {
        return right;
    }
    // Fall back to the word on the left.
    if col == 0 {
        return col;
    }
    let mut left = col;
    while left > 0 && !is_word(bytes[left - 1]) {
        left -= 1;
    }
    if left > 0 {
        return left - 1;
    }
    col
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

/// Byte-col range of the selected portion of `selection_line`, given the
/// ordered selection endpoints `(sel_start, sel_end)`. Returns `None` for
/// lines outside the selection or for empty per-line slices.
pub(crate) fn selection_per_line_range(
    selection_line: usize,
    line_byte_len: usize,
    sel_start: (usize, usize),
    sel_end: (usize, usize),
) -> Option<(usize, usize)> {
    if selection_line < sel_start.0 || selection_line > sel_end.0 {
        return None;
    }
    let col_start = if selection_line == sel_start.0 {
        sel_start.1
    } else {
        0
    };
    let col_end = if selection_line == sel_end.0 {
        sel_end.1
    } else {
        line_byte_len
    };
    if col_start >= col_end {
        None
    } else {
        Some((col_start, col_end))
    }
}

/// Byte offset of `(line, byte_col)` into the document rope. Used by the
/// keyboard handler when it needs to route a typed character through
/// `edit_with_caret` with a byte-range rather than the line/col helpers
/// in the view-model.
fn byte_of(vm: &EditorViewModel, line: usize) -> usize {
    if line >= vm.len_lines() {
        return vm.document_text().len();
    }
    vm.line_byte_range(line).0
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

/// Line count for one "page" of vertical movement (PageUp / PageDown).
/// Roughly the number of visible rows minus a small overlap so the cursor
/// doesn't jump clear of the previous viewport. Pure-logic helper so the
/// page-size math is unit-testable without an `egui::Context`.
pub(crate) fn page_lines_for(visible_rows: usize) -> usize {
    if visible_rows == 0 {
        return 1;
    }
    ((visible_rows as f32 * 0.85).floor() as usize).max(1)
}

/// Snap the viewport scroll target so `line` is visible. If the cursor row
/// is already inside `[visible_first, visible_last)` no scroll change is
/// requested. Otherwise sets `*scroll_target` to the pixel offset that
/// places the cursor at the top (when above) or bottom (when below) of the
/// viewport. The caller passes the proxy through from the DiffView so both
/// panes share one scroll.
pub(crate) fn ensure_line_visible(
    scroll_target: &mut Option<f32>,
    line: usize,
    row_height: f32,
    visible_first: usize,
    visible_last: usize,
) {
    if row_height <= 0.0 || visible_last <= visible_first {
        return;
    }
    if line < visible_first {
        *scroll_target = Some(line as f32 * row_height);
    } else if line >= visible_last {
        // Put cursor at the bottom of the viewport.
        let visible_lines = (visible_last - visible_first) as f32;
        let new_y = (line + 1) as f32 * row_height - visible_lines * row_height;
        *scroll_target = Some(new_y.max(0.0));
    }
    // Otherwise cursor is inside the viewport — no scroll change.
}

// ---------------------------------------------------------------------------
// Per-event cursor helpers. Each takes `&mut line`, `&mut col`,
// `&mut selection: Option<Selection>` and mutates them in place. They never
// touch `self.cursor` directly so they can be unit-tested with no
// `egui::Context`.
// ---------------------------------------------------------------------------

/// Insert `text` at the cursor. If a selection is active, the selection is
/// replaced (mirrors the editor's prior behavior). After the insert, `col`
/// advances by `text.len()` bytes (the cursor is byte-col based).
pub(crate) fn apply_text_insert(
    vm: &mut EditorViewModel,
    text: &str,
    line: &mut usize,
    col: &mut usize,
    selection: &mut Option<drz_viewmodel::Selection>,
) {
    if let Some(sel) = selection.take() {
        let (s, e) = sel.ordered();
        let (nl, nc) = vm.replace_selection_with(s, e, text);
        *line = nl;
        *col = nc;
        return;
    }
    let caret = (*line, *col);
    vm.edit_with_caret(
        byte_of(vm, *line) + *col,
        byte_of(vm, *line) + *col,
        text,
        Some(caret),
    );
    *col += text.len();
    *line = (*line).min(vm.len_lines().saturating_sub(1));
}

/// Backspace: remove the selection if any, else the char to the left of the
/// cursor, else join the current line to the previous one.
pub(crate) fn apply_backspace(
    vm: &mut EditorViewModel,
    line: &mut usize,
    col: &mut usize,
    selection: &mut Option<drz_viewmodel::Selection>,
) {
    if let Some(sel) = selection.take() {
        let (s, e) = sel.ordered();
        let (nl, nc) = vm.replace_selection_with(s, e, "");
        *line = nl;
        *col = nc;
        return;
    }
    if *col > 0 {
        // Use the FRESH line length each call so multi-event backspace on a
        // shrinking line doesn't panic. `floor_col_boundary` was already
        // applied at the top of handle_keys, so `*col` is on a char boundary.
        let prev_char_len = vm.line(*line)[..*col]
            .chars()
            .last()
            .map(|c| c.len_utf8())
            .unwrap_or(1);
        let prev_col = *col - prev_char_len;
        vm.delete_range_line_col((*line, prev_col), (*line, *col));
        *col = prev_col;
    } else if *line > 0 {
        let prev_len = vm.line(*line - 1).len();
        vm.delete_range_line_col((*line - 1, prev_len), (*line, 0));
        *line -= 1;
        *col = prev_len;
    }
}

/// Delete: remove the selection if any, else the char under the cursor,
/// else join the next line into the current one.
pub(crate) fn apply_delete(
    vm: &mut EditorViewModel,
    line: &mut usize,
    col: &mut usize,
    selection: &mut Option<drz_viewmodel::Selection>,
) {
    if let Some(sel) = selection.take() {
        let (s, e) = sel.ordered();
        let (nl, nc) = vm.replace_selection_with(s, e, "");
        *line = nl;
        *col = nc;
        return;
    }
    let line_len = vm.line(*line).len();
    if *col < line_len {
        let next_char_len = vm.line(*line)[*col..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(1);
        vm.delete_range_line_col((*line, *col), (*line, *col + next_char_len));
    } else if *line + 1 < vm.len_lines() {
        let line_byte_len = vm.line(*line).len();
        vm.delete_range_line_col((*line, line_byte_len), (*line + 1, 0));
    }
}

pub(crate) fn apply_arrow_left(
    shift: bool,
    line: &mut usize,
    col: &mut usize,
    selection: &mut Option<drz_viewmodel::Selection>,
) {
    use drz_viewmodel::Selection;
    if shift {
        if selection.is_none() {
            *selection = Some(Selection::new((*line, *col), (*line, *col)));
        }
        if let Some(sel) = selection.as_mut() {
            sel.cursor.1 = sel.cursor.1.saturating_sub(1);
        }
        *col = col.saturating_sub(1);
    } else {
        *selection = None;
        if *col > 0 {
            *col -= 1;
        }
    }
}

pub(crate) fn apply_arrow_right(
    shift: bool,
    line: &mut usize,
    col: &mut usize,
    selection: &mut Option<drz_viewmodel::Selection>,
    line_len: usize,
) {
    use drz_viewmodel::Selection;
    if shift {
        if selection.is_none() {
            *selection = Some(Selection::new((*line, *col), (*line, *col)));
        }
        if let Some(sel) = selection.as_mut() {
            sel.cursor.1 = clamp_col(sel.cursor.1 + 1, line_len);
        }
        *col = clamp_col(*col + 1, line_len);
    } else {
        *selection = None;
        *col = clamp_col(*col + 1, line_len);
    }
}

/// Vertical arrow helper. `up` = ArrowUp (target row = line - 1), else
/// ArrowDown (target row = line + 1). The caller has already bounds-checked
/// so the target row exists in the doc.
pub(crate) fn apply_arrow_vert(
    up: bool,
    shift: bool,
    line: &mut usize,
    col: &mut usize,
    selection: &mut Option<drz_viewmodel::Selection>,
    target_line_len: usize,
) {
    use drz_viewmodel::Selection;
    if shift {
        if selection.is_none() {
            *selection = Some(Selection::new((*line, *col), (*line, *col)));
        }
        if let Some(sel) = selection.as_mut() {
            if up {
                sel.cursor.0 -= 1;
            } else {
                sel.cursor.0 += 1;
            }
            sel.cursor.1 = clamp_col(sel.cursor.1, target_line_len);
        }
    } else {
        *selection = None;
    }
    if up {
        *line -= 1;
    } else {
        *line += 1;
    }
    *col = clamp_col(*col, target_line_len);
}

pub(crate) fn apply_home(
    shift: bool,
    line: &mut usize,
    col: &mut usize,
    selection: &mut Option<drz_viewmodel::Selection>,
) {
    use drz_viewmodel::Selection;
    if shift {
        if selection.is_none() {
            *selection = Some(Selection::new((*line, *col), (*line, *col)));
        }
        if let Some(sel) = selection.as_mut() {
            sel.cursor.1 = 0;
        }
    } else {
        *selection = None;
    }
    *col = 0;
}

pub(crate) fn apply_end(
    shift: bool,
    line_len: usize,
    line: &mut usize,
    col: &mut usize,
    selection: &mut Option<drz_viewmodel::Selection>,
) {
    use drz_viewmodel::Selection;
    let end_col = clamp_col(line_len, line_len);
    if shift {
        if selection.is_none() {
            *selection = Some(Selection::new((*line, *col), (*line, *col)));
        }
        if let Some(sel) = selection.as_mut() {
            sel.cursor.1 = end_col;
        }
    } else {
        *selection = None;
    }
    *col = end_col;
}

/// PageUp: jump the cursor up by `page` lines (clamped to 0). The cursor
/// col is clamped to the new line's length so End-like positioning survives
/// the jump.
pub(crate) fn apply_page_up(
    vm: &mut EditorViewModel,
    page: usize,
    shift: bool,
    line: &mut usize,
    col: &mut usize,
    selection: &mut Option<drz_viewmodel::Selection>,
) {
    use drz_viewmodel::Selection;
    let new_line = line.saturating_sub(page);
    let new_len = if new_line < vm.len_lines() {
        vm.line(new_line).len()
    } else {
        0
    };
    if shift {
        if selection.is_none() {
            *selection = Some(Selection::new((*line, *col), (*line, *col)));
        }
        if let Some(sel) = selection.as_mut() {
            sel.cursor.0 = new_line;
            sel.cursor.1 = clamp_col(sel.cursor.1, new_len);
        }
    } else {
        *selection = None;
    }
    *line = new_line;
    *col = clamp_col(*col, new_len);
}

/// PageDown: jump the cursor down by `page` lines (clamped to last line).
pub(crate) fn apply_page_down(
    vm: &mut EditorViewModel,
    page: usize,
    shift: bool,
    line: &mut usize,
    col: &mut usize,
    selection: &mut Option<drz_viewmodel::Selection>,
) {
    use drz_viewmodel::Selection;
    let last = vm.len_lines().saturating_sub(1);
    let new_line = (*line + page).min(last);
    let new_len = if new_line < vm.len_lines() {
        vm.line(new_line).len()
    } else {
        0
    };
    if shift {
        if selection.is_none() {
            *selection = Some(Selection::new((*line, *col), (*line, *col)));
        }
        if let Some(sel) = selection.as_mut() {
            sel.cursor.0 = new_line;
            sel.cursor.1 = clamp_col(sel.cursor.1, new_len);
        }
    } else {
        *selection = None;
    }
    *line = new_line;
    *col = clamp_col(*col, new_len);
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
    #[allow(unused_assignments)]
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

    #[test]
    fn clamp_col_to_line_len() {
        assert_eq!(clamp_col(10, 4), 4);
        assert_eq!(clamp_col(2, 4), 2);
    }

    #[test]
    fn page_lines_for_uses_eighty_five_percent_of_visible() {
        // PageUp / PageDown move by ~85% of visible rows so the user keeps
        // some context across the jump.
        assert_eq!(page_lines_for(0), 1); // floor of 0 → at least 1
        assert_eq!(page_lines_for(1), 1); // 0.85 → 0 → max(1)
        assert_eq!(page_lines_for(10), 8); // 10 * 0.85 = 8.5 → 8
        assert_eq!(page_lines_for(20), 17); // 20 * 0.85 = 17
        assert_eq!(page_lines_for(100), 85); // 100 * 0.85 = 85
    }

    #[test]
    fn ensure_line_visible_writes_to_proxy_cell() {
        // Old API used a Rc<RefCell<Option<usize>>>; the new API takes
        // `&mut Option<f32>`. The pixel form subsumes the line form:
        // setting `Some(line * row_height)` is identical. We test the
        // pixel form below.
    }

    // -----------------------------------------------------------------
    // ensure_line_visible (pixel form)
    // -----------------------------------------------------------------

    #[test]
    fn ensure_line_visible_no_change_when_cursor_inside_viewport() {
        let mut target: Option<f32> = None;
        ensure_line_visible(&mut target, 5, 18.0, 2, 10);
        assert!(target.is_none());
    }

    #[test]
    fn ensure_line_visible_snaps_when_cursor_above_viewport() {
        let mut target: Option<f32> = None;
        ensure_line_visible(&mut target, 0, 18.0, 5, 12);
        // Cursor above: scroll target = line * row_height = 0 * 18 = 0.
        assert_eq!(target, Some(0.0));
    }

    #[test]
    fn ensure_line_visible_snaps_when_cursor_below_viewport() {
        let mut target: Option<f32> = None;
        // Viewport rows 0..10 → 10 visible lines. Cursor on row 20.
        //   new_y = (20 + 1) * 18 - 10 * 18 = 378 - 180 = 198
        ensure_line_visible(&mut target, 20, 18.0, 0, 10);
        assert_eq!(target, Some(198.0));
    }

    #[test]
    fn ensure_line_visible_handles_zero_viewport() {
        let mut target: Option<f32> = None;
        ensure_line_visible(&mut target, 5, 18.0, 5, 5);
        // degenerate viewport — no change
        assert!(target.is_none());
    }

    // -----------------------------------------------------------------
    // Per-event cursor helpers
    // -----------------------------------------------------------------

    fn fresh_selection() -> Option<drz_viewmodel::Selection> {
        None
    }

    #[test]
    fn apply_text_insert_inserts_at_cursor_and_advances_col() {
        let mut vm = EditorViewModel::from_text("hello\n", drz_viewmodel::LanguageId::PlainText);
        let mut line = 0;
        let mut col = 5;
        let mut sel = fresh_selection();
        apply_text_insert(&mut vm, "X", &mut line, &mut col, &mut sel);
        assert_eq!(vm.document_text(), "helloX\n");
        assert_eq!((line, col), (0, 6));
    }

    #[test]
    fn apply_text_insert_two_calls_insert_both_chars_and_advance_twice() {
        // Regression for the multi-event-per-frame bug: previously both
        // events inserted at the same captured col, overwriting each
        // other.
        let mut vm = EditorViewModel::from_text("ab\n", drz_viewmodel::LanguageId::PlainText);
        let mut line = 0;
        let mut col = 1;
        let mut sel = fresh_selection();
        apply_text_insert(&mut vm, "X", &mut line, &mut col, &mut sel);
        apply_text_insert(&mut vm, "Y", &mut line, &mut col, &mut sel);
        assert_eq!(vm.document_text(), "aXYb\n");
        assert_eq!((line, col), (0, 3));
    }

    #[test]
    fn apply_text_insert_replaces_selection_when_present() {
        let mut vm = EditorViewModel::from_text("hello\n", drz_viewmodel::LanguageId::PlainText);
        let mut line = 0;
        let mut col = 5;
        let mut sel = Some(drz_viewmodel::Selection::new((0, 1), (0, 5)));
        apply_text_insert(&mut vm, "i", &mut line, &mut col, &mut sel);
        // Selection covers bytes 1..5 = "ello"; replaced with "i" → "hi".
        assert_eq!(vm.document_text(), "hi\n");
        assert_eq!((line, col), (0, 2));
        assert!(sel.is_none());
    }

    #[test]
    fn apply_backspace_removes_one_char_and_decrements_col() {
        let mut vm = EditorViewModel::from_text("hello\n", drz_viewmodel::LanguageId::PlainText);
        let mut line = 0;
        let mut col = 5;
        let mut sel = fresh_selection();
        apply_backspace(&mut vm, &mut line, &mut col, &mut sel);
        assert_eq!(vm.document_text(), "hell\n");
        assert_eq!((line, col), (0, 4));
    }

    #[test]
    fn apply_backspace_twice_handles_shrinking_line() {
        // Regression: the previous version captured `col` ONCE and used
        // the stale value for the second event, causing `vm.line(line)[..col]`
        // to slice past the now-shorter line — panic in many cases.
        let mut vm = EditorViewModel::from_text("hello\n", drz_viewmodel::LanguageId::PlainText);
        let mut line = 0;
        let mut col = 5;
        let mut sel = fresh_selection();
        apply_backspace(&mut vm, &mut line, &mut col, &mut sel);
        apply_backspace(&mut vm, &mut line, &mut col, &mut sel);
        assert_eq!(vm.document_text(), "hel\n");
        assert_eq!((line, col), (0, 3));
    }

    #[test]
    fn apply_backspace_at_line_start_joins_previous_line() {
        let mut vm =
            EditorViewModel::from_text("hello\nworld\n", drz_viewmodel::LanguageId::PlainText);
        let mut line = 1;
        let mut col = 0;
        let mut sel = fresh_selection();
        apply_backspace(&mut vm, &mut line, &mut col, &mut sel);
        assert_eq!(vm.document_text(), "helloworld\n");
        assert_eq!((line, col), (0, 5));
    }

    #[test]
    fn apply_delete_removes_char_at_cursor() {
        let mut vm = EditorViewModel::from_text("hello\n", drz_viewmodel::LanguageId::PlainText);
        let mut line = 0;
        let mut col = 1;
        let mut sel = fresh_selection();
        apply_delete(&mut vm, &mut line, &mut col, &mut sel);
        assert_eq!(vm.document_text(), "hllo\n");
        assert_eq!((line, col), (0, 1));
    }

    #[test]
    fn apply_arrow_left_at_zero_col_stays_put() {
        let mut vm = EditorViewModel::from_text("hello\n", drz_viewmodel::LanguageId::PlainText);
        let mut line = 0;
        let mut col = 0;
        let mut sel = fresh_selection();
        apply_arrow_left(false, &mut line, &mut col, &mut sel);
        assert_eq!((line, col), (0, 0));
    }

    #[test]
    fn apply_arrow_left_with_shift_extends_selection() {
        let mut vm = EditorViewModel::from_text("hello\n", drz_viewmodel::LanguageId::PlainText);
        let mut line = 0;
        let mut col = 5;
        let mut sel = fresh_selection();
        apply_arrow_left(true, &mut line, &mut col, &mut sel);
        assert_eq!(col, 4);
        // Selection now covers [(0,5), (0,4)] ordered as [(0,4),(0,5)].
        let s = sel.expect("selection set");
        assert_eq!(s.ordered(), ((0, 4), (0, 5)));
    }

    #[test]
    fn apply_arrow_right_clamps_to_line_end() {
        let mut vm = EditorViewModel::from_text("hi\n", drz_viewmodel::LanguageId::PlainText);
        let mut line = 0;
        let mut col = 2;
        let mut sel = fresh_selection();
        apply_arrow_right(false, &mut line, &mut col, &mut sel, 2);
        assert_eq!(col, 2); // already at end
        apply_arrow_right(false, &mut line, &mut col, &mut sel, 2);
        assert_eq!(col, 2);
    }

    #[test]
    fn apply_arrow_vert_down_clamps_col_to_target_line_len() {
        let mut vm =
            EditorViewModel::from_text("hello\nhi\n", drz_viewmodel::LanguageId::PlainText);
        let mut line = 0;
        let mut col = 5;
        let mut sel = fresh_selection();
        // Target line 1 has len 2; cursor col 5 clamps to 2.
        apply_arrow_vert(false, false, &mut line, &mut col, &mut sel, 2);
        assert_eq!((line, col), (1, 2));
    }

    #[test]
    fn apply_arrow_vert_up_clamps_col_to_target_line_len() {
        let mut vm =
            EditorViewModel::from_text("hi\nhello\n", drz_viewmodel::LanguageId::PlainText);
        let mut line = 1;
        let mut col = 5;
        let mut sel = fresh_selection();
        apply_arrow_vert(true, false, &mut line, &mut col, &mut sel, 2);
        assert_eq!((line, col), (0, 2));
    }

    #[test]
    fn apply_home_moves_col_to_zero_and_clears_selection() {
        let mut vm = EditorViewModel::from_text("hello\n", drz_viewmodel::LanguageId::PlainText);
        let mut line = 0;
        let mut col = 3;
        let mut sel = Some(drz_viewmodel::Selection::new((0, 0), (0, 3)));
        apply_home(false, &mut line, &mut col, &mut sel);
        assert_eq!(col, 0);
        assert!(sel.is_none());
    }

    #[test]
    fn apply_home_with_shift_extends_selection() {
        let mut vm = EditorViewModel::from_text("hello\n", drz_viewmodel::LanguageId::PlainText);
        let mut line = 0;
        let mut col = 3;
        let mut sel = fresh_selection();
        apply_home(true, &mut line, &mut col, &mut sel);
        let s = sel.expect("selection set");
        assert_eq!(s.ordered().0, (0, 0));
        assert_eq!(s.ordered().1, (0, 3));
    }

    #[test]
    fn apply_end_moves_col_to_line_end() {
        let mut vm = EditorViewModel::from_text("hello\n", drz_viewmodel::LanguageId::PlainText);
        let mut line = 0;
        let mut col = 1;
        let mut sel = fresh_selection();
        apply_end(false, 5, &mut line, &mut col, &mut sel);
        assert_eq!(col, 5);
    }

    #[test]
    fn apply_page_up_decrements_line_by_page() {
        let mut vm = EditorViewModel::from_text(
            "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\nk\nl\nm\nn\no\np\n",
            drz_viewmodel::LanguageId::PlainText,
        );
        let mut line = 15;
        let mut col = 0;
        let mut sel = fresh_selection();
        apply_page_up(&mut vm, 5, false, &mut line, &mut col, &mut sel);
        assert_eq!(line, 10);
    }

    #[test]
    fn apply_page_down_increments_line_by_page() {
        let mut vm = EditorViewModel::from_text(
            "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\nk\nl\nm\nn\no\np\n",
            drz_viewmodel::LanguageId::PlainText,
        );
        let mut line = 0;
        let mut col = 0;
        let mut sel = fresh_selection();
        apply_page_down(&mut vm, 5, false, &mut line, &mut col, &mut sel);
        assert_eq!(line, 5);
    }

    #[test]
    fn apply_page_down_clamps_to_last_line() {
        let mut vm = EditorViewModel::from_text("a\nb\nc\n", drz_viewmodel::LanguageId::PlainText);
        let mut line = 0;
        let mut col = 0;
        let mut sel = fresh_selection();
        apply_page_down(&mut vm, 100, false, &mut line, &mut col, &mut sel);
        // ropey keeps a phantom trailing line, so len_lines() == 4 and
        // last = 3.
        assert_eq!(line, 3);
    }

    #[test]
    fn apply_page_up_saturates_at_zero() {
        let mut vm = EditorViewModel::from_text("a\nb\nc\n", drz_viewmodel::LanguageId::PlainText);
        let mut line = 1;
        let mut col = 0;
        let mut sel = fresh_selection();
        apply_page_up(&mut vm, 100, false, &mut line, &mut col, &mut sel);
        assert_eq!(line, 0);
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

    #[test]
    fn word_bound_left_right_alphanumeric_underscore() {
        // "foo bar_baz.qux 42" → click at col 5 (in middle of "bar_baz")
        let line = "foo bar_baz.qux 42";
        // col 5 ('a' in "bar"): word extends over "_" → "bar_baz".
        assert_eq!(word_range(line, 5), (4, 11));
        // col 8 ('b' in "baz"): scan left over '_' into "bar_baz".
        assert_eq!(word_range(line, 8), (4, 11));
        // col 12 ('q' in "qux"): right scan stops at space.
        assert_eq!(word_range(line, 12), (12, 15));
        // col 16 ('4' in "42"): right scan hits EOL.
        assert_eq!(word_range(line, 16), (16, 18));
        // col 20 (past end): clamped → empty at end-of-line.
        assert_eq!(word_range(line, 20), (18, 18));
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
        // "  café  " — c=1B, a=1B, f=1B, é=2B (lead C3 + cont A9).
        // Click at col 4 (inside "caf") → strict impl stops at non-ASCII.
        let line = "  café  ";
        assert_eq!(word_range(line, 4), (2, 5)); // bytes 2..5 == "caf"
                                                 // Click on 'é' (col 5, UTF-8 lead byte) is not an ASCII word char.
        assert_eq!(word_range(line, 5), (5, 5));
    }

    #[test]
    fn snap_to_nearest_word_recovers_from_space_click() {
        // "foo bar baz" → f(0) o(1) o(2) space(3) b(4) a(5) r(6) space(7) b(8) a(9) z(10)
        let line = "foo bar baz";
        // Already on a word: identity.
        assert_eq!(snap_to_nearest_word(line, 4), 4);
        // On the space between "foo" and "bar": prefer right.
        assert_eq!(snap_to_nearest_word(line, 3), 4);
        // On the space between "bar" and "baz": prefer right.
        assert_eq!(snap_to_nearest_word(line, 7), 8);
        // End-of-line fallback: left.
        assert_eq!(snap_to_nearest_word(line, 11), 10);
        // Empty line: returns the input col.
        assert_eq!(snap_to_nearest_word("", 0), 0);
    }

    #[test]
    fn selection_per_line_range_handles_multi_line_and_edges() {
        // Multi-line selection [(0,2)..(2,5)].
        // Start line 0 → [col_start, line_byte_len].
        assert_eq!(selection_per_line_range(0, 8, (0, 2), (2, 5)), Some((2, 8)));
        // Middle line 1 → full line.
        assert_eq!(
            selection_per_line_range(1, 10, (0, 2), (2, 5)),
            Some((0, 10))
        );
        // End line 2 → [0, col_end] — selecting to EOL is a valid range.
        assert_eq!(selection_per_line_range(2, 8, (0, 2), (2, 5)), Some((0, 5)));
        // Single-line selection.
        assert_eq!(selection_per_line_range(0, 8, (0, 2), (0, 5)), Some((2, 5)));
        // Outside the selection → None.
        assert_eq!(selection_per_line_range(3, 8, (0, 2), (2, 5)), None);
        // Start-line endpoint lands on line boundary → empty per-line slice.
        assert_eq!(selection_per_line_range(0, 8, (0, 8), (2, 5)), None);
    }

    // -------------------------------------------------------------------
    // Regression tests for the 3 bugs reported on develop after merge:
    //   1. Shift+Arrow focus jumping to merge-arrow buttons.
    //   2. Double-click not selecting the word.
    //   3. Triple-click not selecting the line.
    //
    // Bug 1 (focus lock) is fixed via `set_focus_lock_filter` in the mouse
    // block; that code path requires an `egui::Context` and is not directly
    // unit-testable. The fix is verified by manual smoke (see task plan).
    //
    // Bugs 2 & 3 are fixed by routing through `apply_double_click` and
    // `apply_triple_click` (extracted as methods precisely so they can be
    // tested headlessly below).
    // -------------------------------------------------------------------

    use drz_viewmodel::{EditorViewModel, LanguageId};

    fn make_vm(text: &str) -> EditorViewModel {
        EditorViewModel::from_text(text, LanguageId::PlainText)
    }

    #[test]
    fn double_click_handler_selects_word_at_clicked_col() {
        // Bug 2 regression: clicking at the middle of a word selects the
        // entire word. The user's "double-click" gesture is a sequence of
        // (click, double-click); this test exercises the double-click path.
        let mut editor = CodeEditor::new();
        let vm = make_vm("hello world\n");
        editor.apply_double_click(0, 2, &vm); // click on 'l' of "hello"
        let sel = editor.selection().expect("selection set");
        let (s, e) = sel.ordered();
        assert_eq!(s, (0, 0));
        assert_eq!(e, (0, 5));
        assert_eq!(editor.cursor(), (0, 5));
    }

    #[test]
    fn double_click_handler_survives_third_click_in_selection() {
        // Bug 2 regression (the precise failure mode users hit): on click 3
        // of what the user *intended* as a double-click, egui tags the click
        // as `triple_clicked`. With the new branch ordering
        // (triple → double → click_by_primary), click 3 now routes to
        // `apply_triple_click`, and the word selection from click 2 is
        // replaced by the line selection. This test simulates click 2 (word)
        // then click 3 (line) and verifies the final state.
        let mut editor = CodeEditor::new();
        let vm = make_vm("hello world\n");
        editor.apply_double_click(0, 2, &vm); // click 2: word "hello"
        let word_sel = editor.selection().expect("word selection set");
        let (s, _e) = word_sel.ordered();
        assert_eq!(s, (0, 0));
        assert_eq!(_e, (0, 5));
        // Click 3: triple-click replaces with line selection ("hello world",
        // 11 bytes; trailing \n excluded).
        editor.apply_triple_click(0, &vm);
        let line_sel = editor.selection().expect("line selection set");
        let (s, e) = line_sel.ordered();
        assert_eq!(s, (0, 0));
        assert_eq!(e, (0, 11));
    }

    #[test]
    fn double_click_handler_snap_to_word_from_space() {
        // A click on whitespace recovers to the nearest word.
        let mut editor = CodeEditor::new();
        let vm = make_vm("hello world\n");
        editor.apply_double_click(0, 5, &vm); // click on the space between words
        let sel = editor.selection().expect("selection set");
        let (s, e) = sel.ordered();
        // Should snap to one of the two adjacent words (either is acceptable
        // per `snap_to_nearest_word` semantics).
        assert!(s == (0, 0) && e == (0, 5) || s == (0, 6) && e == (0, 11));
    }

    #[test]
    fn triple_click_handler_selects_entire_line() {
        // Bug 3 regression: clicking three times selects the whole line
        // content (not including the trailing newline).
        let mut editor = CodeEditor::new();
        let vm = make_vm("hello world\n");
        editor.apply_triple_click(0, &vm);
        let sel = editor.selection().expect("selection set");
        let (s, e) = sel.ordered();
        assert_eq!(s, (0, 0));
        assert_eq!(e, (0, 11));
        assert_eq!(editor.cursor(), (0, 11));
    }

    #[test]
    fn triple_click_handler_works_on_multi_line_doc() {
        let mut editor = CodeEditor::new();
        let vm = make_vm("first\nsecond\nthird\n");
        editor.apply_triple_click(1, &vm); // triple-click on "second"
        let sel = editor.selection().expect("selection set");
        let (s, e) = sel.ordered();
        assert_eq!(s, (1, 0));
        assert_eq!(e, (1, 6));
    }

    #[test]
    fn triple_click_clears_double_click_state() {
        // After a triple-click, `last_double_click` is cleared so a
        // subsequent double-click elsewhere isn't misinterpreted as the
        // third click of a continuing triple.
        let mut editor = CodeEditor::new();
        let vm = make_vm("aaa bbb\n");
        editor.apply_double_click(0, 1, &vm);
        editor.apply_triple_click(0, &vm);
        // Now double-click on line 0 col 5 ('b' of "bbb").
        editor.apply_double_click(0, 5, &vm);
        let sel = editor
            .selection()
            .expect("selection set after second double-click");
        let (s, e) = sel.ordered();
        assert_eq!(s, (0, 4));
        assert_eq!(e, (0, 7));
    }

    #[test]
    fn double_click_on_non_first_line_uses_line_relative_cols() {
        // Regression for the bug found via manual smoke on develop:
        // `apply_double_click` previously added `vm.line_byte_range(line).0`
        // (the document-byte start of the line) to `word_range`'s
        // already-line-relative offsets, producing a Selection with cols
        // that pointed past the end of the line. The paint pass then
        // returned `None` and no highlight rendered. This test exercises a
        // line whose document start byte is non-zero.
        let mut editor = CodeEditor::new();
        let vm = make_vm("first line\nsecond line here\nthird\n");
        // Double-click on 'e' of "second" (col 2 of line 1).
        editor.apply_double_click(1, 2, &vm);
        let sel = editor.selection().expect("selection set");
        let (s, e) = sel.ordered();
        assert_eq!(
            (s, e),
            ((1, 0), (1, 6)),
            "selection must be line-relative cols, not document-byte offsets"
        );
        assert_eq!(editor.cursor(), (1, 6));
    }

    #[test]
    fn double_click_word_text_is_extractable_via_vm() {
        // End-to-end check that `vm.text_in_range` over the selection
        // produced by `apply_double_click` returns the expected word, not
        // empty string. (Catches the same Selection-coords bug as the test
        // above, but at the downstream consumer.)
        let mut editor = CodeEditor::new();
        let vm = make_vm("first line\nsecond line here\nthird\n");
        editor.apply_double_click(1, 2, &vm);
        let sel = editor.selection().expect("selection set");
        let (s, e) = sel.ordered();
        assert_eq!(vm.text_in_range(s, e), "second");
    }
}
