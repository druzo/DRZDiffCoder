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
