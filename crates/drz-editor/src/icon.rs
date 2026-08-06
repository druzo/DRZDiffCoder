const COPY_SVG: &[u8] = include_bytes!("../../../icons/doc.on.clipboard.svg");
const CUT_SVG: &[u8] = include_bytes!("../../../icons/scissors.svg");
const PASTE_SVG: &[u8] = include_bytes!("../../../icons/doc.on.clipboard.fill.svg");

const ICON_PX: u32 = 14;

pub struct EditorIcons {
    copy: Option<egui::TextureHandle>,
    cut: Option<egui::TextureHandle>,
    paste: Option<egui::TextureHandle>,
}

impl EditorIcons {
    pub fn new() -> Self {
        Self {
            copy: None,
            cut: None,
            paste: None,
        }
    }

    pub fn ensure_textures(&mut self, ctx: &egui::Context) {
        if self.copy.is_none() {
            self.copy = rasterize(ctx, "drz_icon_copy", COPY_SVG);
        }
        if self.cut.is_none() {
            self.cut = rasterize(ctx, "drz_icon_cut", CUT_SVG);
        }
        if self.paste.is_none() {
            self.paste = rasterize(ctx, "drz_icon_paste", PASTE_SVG);
        }
    }

    pub fn copy(&self) -> Option<&egui::TextureHandle> {
        self.copy.as_ref()
    }
    pub fn cut(&self) -> Option<&egui::TextureHandle> {
        self.cut.as_ref()
    }
    pub fn paste(&self) -> Option<&egui::TextureHandle> {
        self.paste.as_ref()
    }
}

impl Default for EditorIcons {
    fn default() -> Self {
        Self::new()
    }
}

fn rasterize(ctx: &egui::Context, name: &str, svg_bytes: &[u8]) -> Option<egui::TextureHandle> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(svg_bytes, &opt).ok()?;
    let size = tree.size().width().max(tree.size().height()).max(1.0);
    let scale = ICON_PX as f32 / size;
    let pixmap_w = ICON_PX;
    let pixmap_h = ICON_PX;
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_w, pixmap_h)?;
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let color = egui::ColorImage::from_rgba_unmultiplied(
        [pixmap_w as usize, pixmap_h as usize],
        pixmap.data(),
    );
    Some(ctx.load_texture(name, color, egui::TextureOptions::LINEAR))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TINY_SVG: &[u8] =
        b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 10 10\"><rect width=\"10\" height=\"10\" fill=\"black\"/></svg>";

    #[test]
    fn usvg_accepts_minimal_svg() {
        let opt = usvg::Options::default();
        let tree = usvg::Tree::from_data(TINY_SVG, &opt);
        let tree = tree.expect("usvg must parse the minimal SVG fixture");
        assert!(tree.size().width() > 0.0);
        assert!(tree.size().height() > 0.0);
    }

    #[test]
    fn editor_icons_new_has_no_textures() {
        let icons = EditorIcons::new();
        assert!(icons.copy().is_none());
        assert!(icons.cut().is_none());
        assert!(icons.paste().is_none());
    }

    #[test]
    fn default_impl_matches_new() {
        let a = EditorIcons::new();
        let b = EditorIcons::default();
        assert_eq!(a.copy().is_none(), b.copy().is_none());
        assert_eq!(a.cut().is_none(), b.cut().is_none());
        assert_eq!(a.paste().is_none(), b.paste().is_none());
    }
}
