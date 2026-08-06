use eframe::egui;

/// PNG bytes for the app icon, embedded at compile time so the binary is
/// self-contained and works regardless of CWD / install layout.
const APP_ICON_PNG: &[u8] = include_bytes!("../../../icons/AppIcon.png");

/// Decode the embedded AppIcon into RGBA bytes + dimensions for the OS
/// window icon. Falls back to a tiny solid-color icon if decoding fails so
/// the app still launches headlessly / with no usable image.
pub fn window_icon() -> egui::IconData {
    match eframe::icon_data::from_png_bytes(APP_ICON_PNG) {
        Ok(icon) => icon,
        Err(_) => fallback_icon(),
    }
}

/// Decode the embedded AppIcon into an egui texture handle for in-app
/// rendering (brand bar, welcome view). `None` if decode failed; callers
/// fall back to text.
pub fn load_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let img = image::load_from_memory(APP_ICON_PNG).ok()?.to_rgba8();
    let (w, h) = (img.width() as usize, img.height() as usize);
    let color = egui::ColorImage::from_rgba_unmultiplied([w, h], img.as_raw());
    Some(ctx.load_texture("drz_app_icon", color, egui::TextureOptions::LINEAR))
}

/// Self-install desktop integration on Linux: writes the embedded PNG to
/// the user's hicolor icon theme and a `.desktop` entry pointing at the
/// current executable. Required for the icon to show in the Wayland
/// taskbar/dock (Wayland ignores `with_icon`; the compositor resolves
/// icons via app_id → desktop file → icon theme).
///
/// Idempotent: always rewrites, so an updated binary refreshes its own
/// entry. Best-effort: any failure is silently ignored (app still runs).
pub fn install_desktop_integration() {
    install_desktop_integration_impl().ok();
}

fn install_desktop_integration_impl() -> Result<(), Box<dyn std::error::Error>> {
    let home = std::env::var_os("HOME").ok_or("no HOME")?;
    let home = std::path::PathBuf::from(home);
    let data = home.join(".local/share");

    // Decode embedded PNG once.
    let img = image::load_from_memory(APP_ICON_PNG)?;
    let rgba = img.to_rgba8();

    // Install a few sizes — GNOME picks the best fit.
    for size in [48u32, 128, 256] {
        let dir = data
            .join("icons/hicolor")
            .join(format!("{size}x{size}"))
            .join("apps");
        std::fs::create_dir_all(&dir)?;
        let resized =
            image::imageops::resize(&rgba, size, size, image::imageops::FilterType::Lanczos3);
        resized.save(dir.join("drzdiff.png"))?;
    }

    // Desktop entry — Exec points at the currently running binary so the
    // launcher always opens the same binary the user ran.
    let exe = std::env::current_exe()?;
    let desktop = format!(
        "[Desktop Entry]\n\
         Name=DRZ Diff\n\
         Comment=Source code diff comparer\n\
         Exec={} %U\n\
         Icon=drzdiff\n\
         Type=Application\n\
         Terminal=false\n\
         Categories=Development;Utility;\n\
         StartupWMClass=drzdiff\n",
        exe.display()
    );
    let apps_dir = data.join("applications");
    std::fs::create_dir_all(&apps_dir)?;
    std::fs::write(apps_dir.join("drzdiff.desktop"), desktop)?;

    // Refresh caches if the tools exist (ignore failures — relog also works).
    for cmd in [
        format!("update-desktop-database {}", apps_dir.display()),
        format!(
            "gtk-update-icon-cache -q {} 2>/dev/null",
            data.join("icons/hicolor").display()
        ),
    ] {
        let _ = std::process::Command::new("sh").arg("-c").arg(cmd).output();
    }
    Ok(())
}

fn fallback_icon() -> egui::IconData {
    // 16x16 navy square w/ magenta dot — keeps the OS happy if PNG decode
    // ever fails on an exotic target.
    let size: u32 = 16;
    let mut buf = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let is_dot = (6..=9).contains(&x) && (6..=9).contains(&y);
            let (r, g, b) = if is_dot {
                (232, 121, 249)
            } else {
                (11, 16, 32)
            };
            buf.extend_from_slice(&[r, g, b, 255]);
        }
    }
    egui::IconData {
        rgba: buf,
        width: size,
        height: size,
    }
}
