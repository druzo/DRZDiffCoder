use crate::RowBg;
use drz_viewmodel::types::Style;

pub fn style_color(style: Style, dark: bool) -> egui::Color32 {
    use egui::Color32;
    match (style, dark) {
        (Style::Keyword, true) => Color32::from_rgb(198, 120, 221),
        (Style::Keyword, false) => Color32::from_rgb(166, 38, 164),
        (Style::StringLit, true) => Color32::from_rgb(152, 195, 121),
        (Style::StringLit, false) => Color32::from_rgb(80, 161, 79),
        (Style::Comment, true) => Color32::from_rgb(128, 132, 144),
        (Style::Comment, false) => Color32::from_rgb(160, 160, 160),
        (Style::Function, true) => Color32::from_rgb(97, 175, 239),
        (Style::Function, false) => Color32::from_rgb(64, 120, 242),
        (Style::Type, true) => Color32::from_rgb(229, 192, 123),
        (Style::Type, false) => Color32::from_rgb(193, 132, 1),
        (Style::Number, true) => Color32::from_rgb(209, 154, 102),
        (Style::Number, false) => Color32::from_rgb(182, 86, 17),
        (Style::Constant, true) => Color32::from_rgb(86, 182, 194),
        (Style::Constant, false) => Color32::from_rgb(1, 132, 188),
        (Style::Default, true) => Color32::from_rgb(220, 223, 228),
        (Style::Default, false) => Color32::from_rgb(56, 58, 66),
    }
}

/// Whole-row background tint for diff lines.
pub fn line_bg(bg: RowBg, dark: bool) -> egui::Color32 {
    use egui::Color32;
    match (bg, dark) {
        (RowBg::Added, true) => Color32::from_rgba_unmultiplied(40, 167, 69, 55),
        (RowBg::Added, false) => Color32::from_rgba_unmultiplied(40, 167, 69, 45),
        (RowBg::Removed, true) => Color32::from_rgba_unmultiplied(220, 53, 69, 55),
        (RowBg::Removed, false) => Color32::from_rgba_unmultiplied(220, 53, 69, 45),
    }
}

/// Stronger intra-line emphasis for changed character ranges within a diff
/// line. Sits on top of the line background.
pub fn inline_bg(bg: RowBg, dark: bool) -> egui::Color32 {
    use egui::Color32;
    match (bg, dark) {
        (RowBg::Added, true) => Color32::from_rgba_unmultiplied(40, 167, 69, 120),
        (RowBg::Added, false) => Color32::from_rgba_unmultiplied(46, 160, 67, 110),
        (RowBg::Removed, true) => Color32::from_rgba_unmultiplied(220, 53, 69, 120),
        (RowBg::Removed, false) => Color32::from_rgba_unmultiplied(207, 34, 46, 110),
    }
}
