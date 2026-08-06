mod app;
mod icon;
mod theme;
mod welcome;

use clap::Parser;

#[derive(Parser)]
#[command(name = "drzdiff", about = "DRZ Diff — source code diff tool")]
struct Cli {
    /// Left file
    left: Option<std::path::PathBuf>,
    /// Right file
    right: Option<std::path::PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut vm = drz_viewmodel::AppViewModel::empty();
    if let (Some(l), Some(r)) = (cli.left, cli.right) {
        vm.open_pair_command(&l, &r);
    }
    // Self-install desktop integration (Linux only; best-effort no-op
    // elsewhere). Makes the Wayland taskbar/dock icon resolve via
    // app_id → .desktop → icon theme.
    icon::install_desktop_integration();
    let icon = std::sync::Arc::new(icon::window_icon());
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([720.0, 480.0])
            .with_title("DRZ Diff")
            .with_app_id("drzdiff")
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native(
        "drzdiff",
        options,
        Box::new(move |cc| {
            // Default to dark mode — matches the AppIcon aesthetic. User can
            // toggle to light via the toolbar; the choice persists in
            // eframe::Storage.
            let mut app = app::DrzApp::new(vm, cc);
            // Pull any persisted preference and apply it on top.
            if let Some(storage) = cc.storage {
                let dark = storage
                    .get_string("drz_theme_dark")
                    .map(|v| v != "false")
                    .unwrap_or(true);
                app.set_dark(dark);
            }
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}
