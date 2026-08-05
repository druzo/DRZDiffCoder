mod app;

use clap::Parser;

#[derive(Parser)]
#[command(name = "drzdiff", about = "DRZDiffCoder — source code diff tool")]
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
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "DRZDiffCoder",
        options,
        Box::new(move |cc| Ok(Box::new(app::DrzApp::new(vm, cc)))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}
