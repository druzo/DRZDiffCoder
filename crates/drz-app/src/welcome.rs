use eframe::egui;

/// Centered welcome state shown when no file pair is open. Drives the
/// primary CTA and reacts to drag-and-drop of two files anywhere on the
/// window.
pub struct WelcomeView;

impl WelcomeView {
    pub fn show(
        ui: &mut egui::Ui,
        icon: Option<&egui::TextureHandle>,
        on_open: &mut dyn FnMut(),
    ) {
        let dark = ui.visuals().dark_mode;
        let fg = if dark {
            egui::Color32::from_rgb(230, 233, 244)
        } else {
            egui::Color32::from_rgb(28, 32, 48)
        };
        let dim = if dark {
            egui::Color32::from_rgb(148, 156, 178)
        } else {
            egui::Color32::from_rgb(96, 102, 120)
        };
        let accent = egui::Color32::from_rgb(34, 211, 238);

        ui.vertical_centered(|ui| {
            ui.add_space(48.0);
            if let Some(tex) = icon {
                let size = egui::vec2(140.0, 140.0);
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                ui.painter().image(
                    tex.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            } else {
                ui.add_space(140.0);
            }
            ui.add_space(16.0);

            ui.label(
                egui::RichText::new("DRZ Diff")
                    .font(egui::FontId::proportional(36.0))
                    .strong()
                    .color(fg),
            );
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new("Source code diff comparer")
                    .font(egui::FontId::proportional(14.0))
                    .color(dim),
            );
            ui.add_space(28.0);

            // Primary CTA — match toolbar accent.
            let btn = egui::Button::new(
                egui::RichText::new("Open files…")
                    .font(egui::FontId::proportional(14.0))
                    .color(egui::Color32::WHITE),
            )
            .fill(accent)
            .corner_radius(egui::CornerRadius::same(8))
            .min_size(egui::vec2(180.0, 38.0));
            if ui.add(btn).clicked() {
                on_open();
            }

            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("or drop two files anywhere")
                    .font(egui::FontId::proportional(12.0))
                    .color(dim),
            );

            ui.add_space(28.0);
            ui.label(
                egui::RichText::new("or run:")
                    .font(egui::FontId::proportional(12.0))
                    .color(dim),
            );
            ui.label(
                egui::RichText::new("drzdiff LEFT RIGHT")
                    .font(egui::FontId::monospace(13.0))
                    .color(fg),
            );
        });
    }
}
