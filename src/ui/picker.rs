use crate::app::ForgeDesk;
use eframe::egui::{self, RichText};

impl ForgeDesk {
    pub(in crate::app) fn ui_drop_hint(&self, context: &egui::Context) {
        if context.input(|input| !input.raw.hovered_files.is_empty()) {
            egui::Area::new(egui::Id::new("ui_config_drop_hint"))
                .order(egui::Order::Tooltip)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(context, |ui| {
                    egui::Frame::popup(ui.style())
                        .inner_margin(24)
                        .show(ui, |ui| {
                            ui.heading("Drop UI config or pack folder");
                            ui.label("Import it, then choose it from the UI menu.");
                        });
                });
        }
    }

    pub(in crate::app) fn ui_picker(&mut self, ui: &mut egui::Ui, id: &str) {
        let current = self.active_ui.id.clone();
        let mut selected = current.clone();
        let available: Vec<_> = self
            .ui_catalog
            .packs
            .iter()
            .map(|pack| (pack.id.clone(), pack.config.name.clone()))
            .collect();
        let enabled = self.running.is_none()
            && !self.importing_ui
            && self.capture_path.is_none()
            && self
                .login_task
                .as_ref()
                .is_none_or(|task| task.is_finished());
        ui.add_enabled_ui(enabled,|ui| {
            egui::ComboBox::from_id_salt(id).width(112.0).selected_text(format!("UI · {}",self.active_ui.config.name))
                .show_ui(ui,|ui| {
                    for (id,name) in available {ui.selectable_value(&mut selected,id,name);}
                    ui.separator();
                    if ui.button("Custom… Import config").clicked() {self.import_ui(None,ui.ctx());ui.close();}
                    if ui.button("Reload current UI").clicked() {self.switch_ui(&current,ui.ctx());ui.close();}
                    if ui.button("Refresh UI folder").clicked() {self.scan_ui();ui.close();}
                    ui.label(RichText::new("Switching restarts the app.\nModels still use Codex / ChatGPT.").small());
                }).response.on_hover_text("Choose a UI. Configs are discovered from the UI folder. You can also drop a JSON file or pack folder into the window.");
        }).response.on_disabled_hover_text("Finish the current response or sign-in before restarting the UI.");
        if selected != current {
            self.switch_ui(&selected, ui.ctx());
        }
    }
}
