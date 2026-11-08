use crate::app::ReDox;
use eframe::egui;

impl ReDox {
    pub(super) fn update_prompt(&mut self, context: &egui::Context) {
        let repaint = context.clone();
        self.updates.poll(move || repaint.request_repaint());
        let Some(release) = &self.updates.available else {
            return;
        };
        let mut dismiss = false;
        egui::Window::new("Update available")
            .collapsible(false)
            .resizable(false)
            .show(context, |ui| {
                ui.label(format!(
                    "ReDox {} is available. Would you like to update?",
                    release.tag_name
                ));
                ui.horizontal(|ui| {
                    if ui.button("View release and download").clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(&release.html_url));
                        dismiss = true;
                    }
                    dismiss |= ui.button("Later").clicked();
                });
            });
        if dismiss {
            self.updates.available = None;
        }
    }
}
