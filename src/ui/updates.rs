use crate::app::ReDox;
use eframe::egui;

impl ReDox {
    pub(super) fn update_prompt(&mut self, context: &egui::Context) {
        if !self.updates.startup_checked {
            self.updates.startup_checked = true;
            if !self.ui_preferences.hide_update_prompts {
                self.updates.show_prompt = true;
                let repaint = context.clone();
                self.updates.check(move || repaint.request_repaint());
            }
        }
        self.updates.poll();
        if self.running.is_none() && self.updates.finish_install() {
            context.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.updates.show_prompt && self.updates.available.is_some() {
            egui::Window::new("ReDox update available")
                .collapsible(false)
                .resizable(false)
                .show(context, |ui| {
                    ui.label(format!(
                        "ReDox {} is available. Download and restart now?",
                        self.updates.available.as_ref().unwrap().tag_name
                    ));
                    ui.checkbox(
                        &mut self.ui_preferences.hide_update_prompts,
                        "Never show again",
                    );
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !self.updates.checking() && self.running.is_none(),
                                egui::Button::new("Yes"),
                            )
                            .clicked()
                        {
                            let repaint = context.clone();
                            self.updates.install(move || repaint.request_repaint());
                        }
                        if ui.button("No").clicked() {
                            self.updates.show_prompt = false;
                        }
                    });
                    ui.label(&self.updates.status);
                    if self.running.is_some() {
                        ui.label("Finish or stop the current response before updating.");
                    }
                });
        }
    }

    pub(super) fn updates_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Updates");
        ui.label(format!("Installed version: {}", env!("CARGO_PKG_VERSION")));
        ui.label("Check GitHub for the latest ReDox release.");
        ui.checkbox(
            &mut self.ui_preferences.hide_update_prompts,
            "Never show update prompts at startup",
        );
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
        if self.updates.available.is_some()
            && ui
                .add_enabled(
                    !self.updates.checking() && self.running.is_none(),
                    egui::Button::new("Update and restart"),
                )
                .clicked()
        {
            let context = ui.ctx().clone();
            self.updates.install(move || context.request_repaint());
        }
        ui.label("Updating replaces this executable in its current folder and preserves a backup in temp. Chats and settings are retained.");
    }
}
