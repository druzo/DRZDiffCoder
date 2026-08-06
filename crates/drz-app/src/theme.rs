use eframe::egui;

/// Palette anchored to AppIcon.png: neon cyan / magenta / lime over deep
/// navy on dark, paper-pale on light. Single source of truth for the UI.
pub mod palette {
    use eframe::egui::Color32;

    pub const NAVY: Color32 = Color32::from_rgb(11, 16, 32);
    pub const PANEL_DARK: Color32 = Color32::from_rgb(18, 24, 46);
    pub const PANEL_LIGHT: Color32 = Color32::from_rgb(245, 246, 250);

    pub const CYAN: Color32 = Color32::from_rgb(34, 211, 238);
    pub const MAGENTA: Color32 = Color32::from_rgb(232, 121, 249);
    pub const LIME: Color32 = Color32::from_rgb(163, 230, 53);
    pub const AMBER: Color32 = Color32::from_rgb(251, 191, 36);
    pub const ROSE: Color32 = Color32::from_rgb(244, 63, 94);

    // Text
    pub const TEXT_DARK: Color32 = Color32::from_rgb(230, 233, 244);
    pub const TEXT_DIM_DARK: Color32 = Color32::from_rgb(148, 156, 178);
    pub const TEXT_LIGHT: Color32 = Color32::from_rgb(28, 32, 48);
    pub const TEXT_DIM_LIGHT: Color32 = Color32::from_rgb(96, 102, 120);

    // Surfaces
    pub const SURFACE_DARK: Color32 = Color32::from_rgb(22, 28, 50);
    pub const SURFACE_LIGHT: Color32 = Color32::from_rgb(255, 255, 255);
}

/// Build a complete Visuals for DRZ Diff. `dark=true` matches the AppIcon
/// look; `dark=false` is a clean paper-light mode with the same accents.
pub fn drz_visuals(dark: bool) -> egui::Visuals {
    use palette as p;
    let mut v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };

    let (panel, surface, text, text_dim, ext_bg, weak_bg, stroke) = if dark {
        (
            p::PANEL_DARK,
            p::SURFACE_DARK,
            p::TEXT_DARK,
            p::TEXT_DIM_DARK,
            p::NAVY,
            egui::Color32::from_rgb(28, 36, 64),
            egui::Color32::from_rgba_unmultiplied(80, 90, 130, 100),
        )
    } else {
        (
            p::PANEL_LIGHT,
            p::SURFACE_LIGHT,
            p::TEXT_LIGHT,
            p::TEXT_DIM_LIGHT,
            egui::Color32::from_rgb(232, 234, 240),
            egui::Color32::from_rgb(240, 242, 248),
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 30),
        )
    };

    v.override_text_color = Some(text);
    v.extreme_bg_color = ext_bg;
    v.faint_bg_color = weak_bg;
    v.code_bg_color = surface;
    v.panel_fill = panel;

    // WidgetVisuals are mutated in place through the public `widgets` field.
    let r6 = egui::CornerRadius::same(6);
    let mut apply = |w: &mut egui::style::WidgetVisuals,
                     bg: egui::Color32,
                     bg_stroke: egui::Stroke,
                     fg: egui::Stroke| {
        w.bg_fill = bg;
        w.weak_bg_fill = bg;
        w.bg_stroke = bg_stroke;
        w.fg_stroke = fg;
        w.corner_radius = r6;
        w.expansion = 0.0;
    };
    apply(
        &mut v.widgets.noninteractive,
        panel,
        egui::Stroke::new(1.0, stroke),
        egui::Stroke::new(1.0, text_dim),
    );
    apply(
        &mut v.widgets.inactive,
        egui::Color32::from_rgba_unmultiplied(text.r(), text.g(), text.b(), if dark { 18 } else { 14 }),
        egui::Stroke::new(1.0, stroke),
        egui::Stroke::new(1.0, text),
    );
    apply(
        &mut v.widgets.hovered,
        p::CYAN.gamma_multiply(if dark { 0.25 } else { 0.18 }),
        egui::Stroke::new(1.0, p::CYAN),
        egui::Stroke::new(1.0, text),
    );
    apply(
        &mut v.widgets.active,
        p::CYAN.gamma_multiply(if dark { 0.45 } else { 0.32 }),
        egui::Stroke::new(1.0, p::CYAN),
        egui::Stroke::new(1.2, text),
    );
    apply(
        &mut v.widgets.open,
        weak_bg,
        egui::Stroke::new(1.0, p::CYAN),
        egui::Stroke::new(1.0, text),
    );

    v.selection.bg_fill = p::CYAN.gamma_multiply(if dark { 0.35 } else { 0.25 });
    v.selection.stroke = egui::Stroke::new(1.0, p::CYAN);

    v.hyperlink_color = p::CYAN;
    v.warn_fg_color = p::AMBER;
    v.error_fg_color = p::ROSE;

    v.window_corner_radius = egui::CornerRadius::same(8);
    v.menu_corner_radius = egui::CornerRadius::same(6);
    v.window_stroke = egui::Stroke::new(1.0, stroke);

    v
}

/// Style overrides layered on top of `drz_visuals` — spacing, font sizes.
pub fn drz_style(dark: bool) -> egui::Style {
    let mut s = egui::Style::default();
    let mono_size = 14.5;
    s.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::proportional(22.0),
    );
    s.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::proportional(14.0),
    );
    s.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(13.0),
    );
    s.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::monospace(mono_size),
    );
    s.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::proportional(11.0),
    );
    s.spacing.item_spacing = egui::vec2(8.0, 6.0);
    s.spacing.button_padding = egui::vec2(10.0, 5.0);
    s.spacing.window_margin = egui::Margin::same(8);
    s.spacing.scroll.bar_width = 10.0;
    s.spacing.scroll.bar_outer_margin = 2.0;
    s.spacing.scroll.bar_inner_margin = 2.0;
    s.visuals = drz_visuals(dark);
    s
}

/// Helper for painting the divider strip between top panels.
pub fn separator_stroke(ctx: &egui::Context) -> egui::Stroke {
    let dark = ctx.style().visuals.dark_mode;
    egui::Stroke::new(
        1.0,
        if dark {
            egui::Color32::from_rgba_unmultiplied(80, 90, 130, 100)
        } else {
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 30)
        },
    )
}
