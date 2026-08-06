use drz_diff_ui::DiffView;
use drz_viewmodel::AppViewModel;
use std::sync::Arc;

pub struct DrzApp {
    vm: AppViewModel,
    diff_view: DiffView,
}

impl DrzApp {


    fn open_dialogs(&mut self) {
        let left = rfd::FileDialog::new().set_title("Left file").pick_file();
        let right = rfd::FileDialog::new().set_title("Right file").pick_file();
        if let (Some(l), Some(r)) = (left, right) {
            self.vm.open_pair_command(&l, &r);
        }
    }
}

impl eframe::App for DrzApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ensure repaint callback on (re-opened) diff
        // let ctx_clone = ctx.clone();
        if let Some(d) = self.vm.diff_mut() {
            d.set_repaint_callback(Arc::new(move || ctx_clone.request_repaint()));
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(100)); // debounce poll cadence


        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open files…").clicked() {
                        ui.close_menu();
                        self.open_dialogs();
                    }
                    if ui.button("Save all").clicked() {
                        self.vm.save_all();
                    }
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        });

        if let Some(err) = self.vm.error().map(|e| e.to_string()) {
            egui::TopBottomPanel::top("error").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::RED, &err);
                    if ui.button("✕").clicked() {
                        self.vm.dismiss_error();
                    }
                });
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.vm.title()));
            if let Some(d) = self.vm.diff_mut() {
                self.diff_view.show(ui, d);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("File → Open files… or run: drzdiff LEFT RIGHT");
                });
            }
        });
    }
}
