use crate::theme;
use crate::welcome::WelcomeView;
use drz_diff_ui::DiffView;
use drz_viewmodel::AppViewModel;
use std::sync::Arc;

/// PNG bytes for the toolbar Swap button (32×27, white bidirectional arrows
/// on transparent). Embedded so the binary is self-contained.
const SWAP_ICON_PNG: &[u8] = include_bytes!("../../../icons/rectangle.2.swap.png");

pub struct DrzApp {
    vm: AppViewModel,
    diff_view: DiffView,
    /// Cached AppIcon texture for in-app rendering (brand bar + welcome).
    /// `None` if PNG decoding failed.
    brand_icon: Option<egui::TextureHandle>,
    /// Toolbar Swap-button icon. `None` if PNG decoding failed; toolbar
    /// falls back to the text glyph.
    swap_icon: Option<egui::TextureHandle>,
    dark: bool,
}

impl DrzApp {
    pub fn new(mut vm: AppViewModel, cc: &eframe::CreationContext<'_>) -> DrzApp {
        // Apply default DRZ visuals before the first frame.
        let dark = true;
        cc.egui_ctx.set_style(theme::drz_style(dark));

        let ctx = cc.egui_ctx.clone();
        let repaint: Arc<dyn Fn() + Send + Sync> = Arc::new(move || ctx.request_repaint());
        if let Some(d) = vm.diff_mut() {
            d.set_repaint_callback(repaint);
        }
        let brand_icon = crate::icon::load_texture(&cc.egui_ctx);
        let swap_icon = load_swap_icon(&cc.egui_ctx);
        DrzApp {
            vm,
            diff_view: DiffView::new(),
            brand_icon,
            swap_icon,
            dark,
        }
    }

    pub fn set_dark(&mut self, dark: bool) {
        self.dark = dark;
    }

    fn open_dialogs(&mut self) {
        let left = rfd::FileDialog::new().set_title("Left file").pick_file();
        let right = rfd::FileDialog::new().set_title("Right file").pick_file();
        if let (Some(l), Some(r)) = (left, right) {
            self.vm.open_pair_command(&l, &r);
        }
    }

    fn open_paths(&mut self, left: &std::path::Path, right: &std::path::Path) {
        self.vm.open_pair_command(left, right);
    }

    fn swap_sides(&mut self) {
        if let Some(d) = self.vm.diff_mut() {
            d.swap_sides();
        }
    }

    fn reload_from_disk(&mut self) {
        let paths = self.vm.diff().and_then(|d| {
            let l = d.left().path()?.to_path_buf();
            let r = d.right().path()?.to_path_buf();
            Some((l, r))
        });
        if let Some((l, r)) = paths {
            self.vm.open_pair_command(&l, &r);
        }
    }

    fn any_dirty(&self) -> bool {
        self.vm
            .diff()
            .map(|d| d.left().is_dirty() || d.right().is_dirty())
            .unwrap_or(false)
    }

    fn detect_language(vm: &AppViewModel) -> Option<&'static str> {
        let p = vm
            .diff()
            .and_then(|d| d.right().path().or_else(|| d.left().path()))?;
        Some(drz_viewmodel::LanguageId::from_path(p).label())
    }
}

impl eframe::App for DrzApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Re-apply theme if toggled.
        let want_dark = self.dark;
        let cur_dark = ctx.style().visuals.dark_mode;
        let cur_mono = ctx.style().text_styles[&egui::TextStyle::Monospace].size;
        let new_mono = theme::drz_style(want_dark).text_styles[&egui::TextStyle::Monospace].size;
        if cur_dark != want_dark || (cur_mono - new_mono).abs() > f32::EPSILON {
            ctx.set_style(theme::drz_style(want_dark));
        }

        // Repaint callback wired on every frame so a re-opened diff still
        // gets background diff-thread completion repaints.
        let ctx_clone = ctx.clone();
        if let Some(d) = self.vm.diff_mut() {
            d.set_repaint_callback(Arc::new(move || ctx_clone.request_repaint()));
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        // Shortcuts — Ctrl/Cmd+S, Ctrl/Cmd+O, Ctrl/Cmd+Shift+T, Ctrl/Cmd+Q.
        ctx.input(|i| {
            let cmd = i.modifiers.command;
            if cmd && i.key_pressed(egui::Key::S) {
                self.vm.save_all();
            }
            if cmd && i.key_pressed(egui::Key::O) && !i.modifiers.shift {
                self.open_dialogs();
            }
            if cmd && i.modifiers.shift && i.key_pressed(egui::Key::T) {
                self.dark = !self.dark;
            }
            if cmd && i.key_pressed(egui::Key::Q) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });

        // Drag-and-drop on the whole window — take first two files dropped.
        let dropped: Vec<std::path::PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if dropped.len() >= 2 {
            self.open_paths(&dropped[0], &dropped[1]);
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.vm.title()));

        paint_brand_bar(
            ctx,
            self.brand_icon.as_ref(),
            self.dark,
            &mut |dark: bool| {
                self.dark = dark;
            },
        );

        // Clone handles so the borrow on `self` ends before constructing
        // the action closure (which also borrows `self` mutably).
        let swap_icon = self.swap_icon.clone();
        paint_toolbar(
            ctx,
            self.vm.diff().is_some(),
            self.any_dirty(),
            Self::detect_language(&self.vm),
            swap_icon.as_ref(),
            &mut |action| match action {
                ToolAction::Open => self.open_dialogs(),
                ToolAction::Save => self.vm.save_all(),
                ToolAction::Swap => self.swap_sides(),
                ToolAction::Reload => self.reload_from_disk(),
                ToolAction::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            },
        );

        if let Some(diff) = self.vm.diff() {
            paint_path_bar(
                ctx,
                diff.left().path(),
                diff.right().path(),
                diff.left().is_dirty(),
                diff.right().is_dirty(),
            );
        }

        if let Some(err) = self.vm.error().map(|e| e.to_string()) {
            paint_error_banner(ctx, &err, &mut || self.vm.dismiss_error());
        }

        if let Some(diff) = self.vm.diff() {
            paint_status_bar(ctx, diff.stats(), self.dark);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(d) = self.vm.diff_mut() {
                self.diff_view.show(ui, d);
            } else {
                // Clone the handle so we don't hold a borrow of `self` while
                // the open callback is constructed (which also borrows self).
                let icon = self.brand_icon.clone();
                WelcomeView::show(ui, icon.as_ref(), &mut || self.open_dialogs());
            }
        });

        // Persist theme preference.
        if let Some(storage) = frame.storage_mut() {
            storage.set_string(
                "drz_theme_dark",
                if self.dark {
                    "true".to_string()
                } else {
                    "false".to_string()
                },
            );
        }
    }
}

enum ToolAction {
    Open,
    Save,
    Swap,
    Reload,
    Quit,
}

/// Decode the embedded swap-icon PNG into an egui texture, recolored to
/// white. The source PNG is black-on-transparent (alpha only); we rewrite
/// RGB to 255 while preserving alpha so the icon reads as white on the
/// toolbar. `None` if the PNG fails to decode — callers fall back to a
/// text glyph.
fn load_swap_icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let mut img = image::load_from_memory(SWAP_ICON_PNG).ok()?.to_rgba8();
    for px in img.pixels_mut() {
        px.0[0] = 255;
        px.0[1] = 255;
        px.0[2] = 255;
    }
    let (w, h) = (img.width() as usize, img.height() as usize);
    let color = egui::ColorImage::from_rgba_unmultiplied([w, h], img.as_raw());
    Some(ctx.load_texture("drz_swap_icon", color, egui::TextureOptions::LINEAR))
}

fn paint_brand_bar(
    ctx: &egui::Context,
    icon: Option<&egui::TextureHandle>,
    dark: bool,
    on_toggle_dark: &mut dyn FnMut(bool),
) {
    let frame = egui::Frame::default()
        .fill(if dark {
            egui::Color32::from_rgb(14, 18, 38)
        } else {
            egui::Color32::from_rgb(248, 249, 252)
        })
        .inner_margin(egui::Margin::symmetric(14, 8));
    egui::TopBottomPanel::top("brand")
        .frame(frame)
        .resizable(false)
        .exact_height(44.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(tex) = icon {
                    ui.image((tex.id(), egui::vec2(28.0, 28.0)));
                    ui.add_space(8.0);
                }
                ui.label(
                    egui::RichText::new("DRZ Diff")
                        .font(egui::FontId::proportional(17.0))
                        .strong(),
                );
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("v0.1.0")
                        .font(egui::FontId::proportional(11.0))
                        .color(egui::Color32::from_rgb(148, 156, 178)),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let theme_label = if dark { "☀" } else { "☾" };
                    let tip = if dark {
                        "Switch to light theme"
                    } else {
                        "Switch to dark theme"
                    };
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new(theme_label).size(16.0))
                                .frame(false),
                        )
                        .on_hover_text(tip)
                        .clicked()
                    {
                        on_toggle_dark(!dark);
                    }
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Ctrl+Shift+T to toggle")
                            .font(egui::FontId::proportional(11.0))
                            .color(egui::Color32::from_rgb(148, 156, 178)),
                    );
                });
            });
        });
}

fn paint_toolbar(
    ctx: &egui::Context,
    has_diff: bool,
    dirty: bool,
    language: Option<&'static str>,
    swap_icon: Option<&egui::TextureHandle>,
    on_action: &mut dyn FnMut(ToolAction),
) {
    let frame = egui::Frame::default()
        .fill(if ctx.style().visuals.dark_mode {
            egui::Color32::from_rgb(20, 26, 50)
        } else {
            egui::Color32::from_rgb(252, 253, 255)
        })
        .inner_margin(egui::Margin::symmetric(14, 6));
    egui::TopBottomPanel::top("toolbar")
        .frame(frame)
        .resizable(false)
        .exact_height(48.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("📂  Open files…")
                                .color(egui::Color32::WHITE)
                                .size(13.0),
                        )
                        .fill(egui::Color32::from_rgb(34, 211, 238))
                        .corner_radius(egui::CornerRadius::same(6))
                        .min_size(egui::vec2(130.0, 30.0)),
                    )
                    .on_hover_text("Open two files to compare (Ctrl+O)")
                    .clicked()
                {
                    on_action(ToolAction::Open);
                }
                ui.add_space(6.0);

                let save_enabled = has_diff && dirty;
                let save_btn = egui::Button::new(egui::RichText::new("💾  Save").size(13.0).color(
                    if save_enabled {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgb(148, 156, 178)
                    },
                ))
                .corner_radius(egui::CornerRadius::same(6))
                .min_size(egui::vec2(90.0, 30.0));
                let save_resp = ui
                    .add_enabled(save_enabled, save_btn)
                    .on_hover_text("Save both files (Ctrl+S)");
                if save_resp.clicked() {
                    on_action(ToolAction::Save);
                }
                ui.add_space(6.0);

                let swap_label = egui::RichText::new("Swap")
                    .size(13.0)
                    .color(egui::Color32::WHITE);
                let swap_btn = match swap_icon {
                    Some(tex) => {
                        egui::Button::image_and_text((tex.id(), egui::vec2(18.0, 15.0)), swap_label)
                            .corner_radius(egui::CornerRadius::same(6))
                            .min_size(egui::vec2(110.0, 30.0))
                    }
                    None => egui::Button::new(
                        egui::RichText::new("\u{21c4}  Swap")
                            .size(13.0)
                            .color(egui::Color32::WHITE),
                    )
                    .corner_radius(egui::CornerRadius::same(6))
                    .min_size(egui::vec2(110.0, 30.0)),
                };
                if ui
                    .add_enabled(has_diff, swap_btn)
                    .on_hover_text("Swap left \u{2194} right")
                    .clicked()
                {
                    on_action(ToolAction::Swap);
                }
                ui.add_space(6.0);

                let reload_btn = egui::Button::new(
                    egui::RichText::new("⟳  Reload")
                        .size(13.0)
                        .color(egui::Color32::WHITE),
                )
                .corner_radius(egui::CornerRadius::same(6))
                .min_size(egui::vec2(94.0, 30.0));
                if ui
                    .add_enabled(has_diff, reload_btn)
                    .on_hover_text("Re-read both files from disk")
                    .clicked()
                {
                    on_action(ToolAction::Reload);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(lang) = language {
                        ui.label(
                            egui::RichText::new(format!(" {lang} "))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(232, 121, 249)),
                        );
                    }
                });
            });
        });
}

fn paint_path_bar(
    ctx: &egui::Context,
    left: Option<&std::path::Path>,
    right: Option<&std::path::Path>,
    left_dirty: bool,
    right_dirty: bool,
) {
    let frame = egui::Frame::default()
        .fill(if ctx.style().visuals.dark_mode {
            egui::Color32::from_rgb(16, 22, 42)
        } else {
            egui::Color32::from_rgb(245, 246, 250)
        })
        .inner_margin(egui::Margin::symmetric(14, 4));
    egui::TopBottomPanel::top("paths")
        .frame(frame)
        .resizable(false)
        .exact_height(30.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("L")
                        .font(egui::FontId::monospace(11.0))
                        .color(egui::Color32::from_rgb(34, 211, 238))
                        .strong(),
                );
                ui.add_space(4.0);
                let l_name = left
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "(untitled)".into());
                ui.label(
                    egui::RichText::new(l_name)
                        .font(egui::FontId::monospace(12.0))
                        .color(egui::Color32::from_rgb(220, 223, 232)),
                );
                if left_dirty {
                    ui.label(
                        egui::RichText::new("•")
                            .color(egui::Color32::from_rgb(251, 191, 36))
                            .strong(),
                    );
                }
                ui.add_space(6.0);
                ui.label(egui::RichText::new("↔").color(egui::Color32::from_rgb(148, 156, 178)));
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("R")
                        .font(egui::FontId::monospace(11.0))
                        .color(egui::Color32::from_rgb(232, 121, 249))
                        .strong(),
                );
                ui.add_space(4.0);
                let r_name = right
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "(untitled)".into());
                ui.label(
                    egui::RichText::new(r_name)
                        .font(egui::FontId::monospace(12.0))
                        .color(egui::Color32::from_rgb(220, 223, 232)),
                );
                if right_dirty {
                    ui.label(
                        egui::RichText::new("•")
                            .color(egui::Color32::from_rgb(251, 191, 36))
                            .strong(),
                    );
                }
            });
        });
}

fn paint_error_banner(ctx: &egui::Context, msg: &str, on_dismiss: &mut dyn FnMut()) {
    let frame = egui::Frame::default()
        .fill(egui::Color32::from_rgb(76, 18, 30))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(244, 63, 94)))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(14, 8));
    egui::TopBottomPanel::top("error")
        .frame(frame)
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("⚠")
                        .color(egui::Color32::from_rgb(251, 191, 36))
                        .size(16.0),
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(msg)
                        .color(egui::Color32::from_rgb(252, 220, 220))
                        .size(13.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("✕")
                                    .color(egui::Color32::from_rgb(252, 220, 220))
                                    .size(13.0),
                            )
                            .frame(false),
                        )
                        .on_hover_text("Dismiss")
                        .clicked()
                    {
                        on_dismiss();
                    }
                });
            });
        });
}

fn paint_status_bar(ctx: &egui::Context, stats: drz_viewmodel::DiffStats, dark: bool) {
    let frame = egui::Frame::default()
        .fill(if dark {
            egui::Color32::from_rgb(14, 18, 38)
        } else {
            egui::Color32::from_rgb(248, 249, 252)
        })
        .inner_margin(egui::Margin::symmetric(14, 4));
    egui::TopBottomPanel::bottom("status")
        .frame(frame)
        .resizable(false)
        .exact_height(26.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let hunk_word = if stats.hunks == 1 { "hunk" } else { "hunks" };
                ui.label(
                    egui::RichText::new(format!("{} {}", stats.hunks, hunk_word))
                        .font(egui::FontId::proportional(11.0))
                        .color(egui::Color32::from_rgb(148, 156, 178)),
                );
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new(format!("+{}", stats.added))
                        .font(egui::FontId::monospace(11.0))
                        .color(egui::Color32::from_rgb(34, 211, 238))
                        .strong(),
                );
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!("−{}", stats.removed))
                        .font(egui::FontId::monospace(11.0))
                        .color(egui::Color32::from_rgb(232, 121, 249))
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("UTF-8")
                            .font(egui::FontId::proportional(11.0))
                            .color(egui::Color32::from_rgb(148, 156, 178)),
                    );
                });
            });
        });
}
