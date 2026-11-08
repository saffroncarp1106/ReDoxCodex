use crate::app::ReDox;
use eframe::egui;

impl ReDox {
    pub(super) fn update_prompt(&mut self, _context: &egui::Context) {
        self.updates.poll();
    }

    pub(super) fn updates_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Updates");
        ui.label(format!("Installed version: {}", env!("CARGO_PKG_VERSION")));
        ui.label("Check GitHub for the latest ReDox release.");
        if ui
            .add_enabled(
                !self.updates.checking(),
                egui::Button::new("Check for updates"),
            )
            .clicked()
        {
            let context = ui.ctx().clone();
            self.updates.check(move || context.request_repaint());
        }
        if self.updates.checking() {
            ui.spinner();
        }
        if !self.updates.status.is_empty() {
            ui.label(&self.updates.status);
        }
        if let Some(release) = &self.updates.available {
            ui.label(format!("ReDox {} is available.", release.tag_name));
            if ui.button("View release and download").clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab(&release.html_url));
            }
        }
        ui.label("Download and installation are manual.");
    }
}
