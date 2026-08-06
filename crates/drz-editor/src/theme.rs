use crate::RowBg;
use drz_viewmodel::types::Style;

pub fn style_color(style: Style, dark: bool) -> egui::Color32 {
    use egui::Color32;
    match (style, dark) {
        (Style::Keyword, true) => Color32::from_rgb(232, 121, 249),
        (Style::Keyword, false) => Color32::from_rgb(166, 38, 164),
        (Style::StringLit, true) => Color32::from_rgb(163, 230, 53),
        (Style::StringLit, false) => Color32::from_rgb(20, 130, 60),
        (Style::Comment, true) => Color32::from_rgb(120, 130, 150),
        (Style::Comment, false) => Color32::from_rgb(140, 148, 160),
        (Style::Function, true) => Color32::from_rgb(34, 211, 238),
        (Style::Function, false) => Color32::from_rgb(20, 110, 200),
        (Style::Type, true) => Color32::from_rgb(251, 191, 36),
        (Style::Type, false) => Color32::from_rgb(170, 100, 0),
        (Style::Number, true) => Color32::from_rgb(255, 138, 80),
        (Style::Number, false) => Color32::from_rgb(180, 80, 12),
        (Style::Constant, true) => Color32::from_rgb(94, 200, 220),
        (Style::Constant, false) => Color32::from_rgb(0, 120, 170),
        (Style::Default, true) => Color32::from_rgb(220, 223, 232),
        (Style::Default, false) => Color32::from_rgb(40, 44, 56),
    }
}

/// Whole-row background tint for diff lines.
pub fn line_bg(bg: RowBg, dark: bool) -> egui::Color32 {
    use egui::Color32;
    match (bg, dark) {
        (RowBg::Added, true) => Color32::from_rgba_unmultiplied(34, 211, 238, 42),
        (RowBg::Added, false) => Color32::from_rgba_unmultiplied(34, 211, 238, 22),
        (RowBg::Removed, true) => Color32::from_rgba_unmultiplied(232, 121, 249, 42),
        (RowBg::Removed, false) => Color32::from_rgba_unmultiplied(232, 121, 249, 22),
    }
}

/// Stronger intra-line emphasis for changed character ranges within a diff
/// line. Sits on top of the line background.
pub fn inline_bg(bg: RowBg, dark: bool) -> egui::Color32 {
    use egui::Color32;
    match (bg, dark) {
        (RowBg::Added, true) => Color32::from_rgba_unmultiplied(34, 211, 238, 110),
        (RowBg::Added, false) => Color32::from_rgba_unmultiplied(34, 211, 238, 80),
        (RowBg::Removed, true) => Color32::from_rgba_unmultiplied(232, 121, 249, 110),
        (RowBg::Removed, false) => Color32::from_rgba_unmultiplied(232, 121, 249, 80),
    }
}
