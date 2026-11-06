use crate::app::ForgeDesk;
use eframe::egui;

impl ForgeDesk {
    pub(in crate::app) fn rename_dialog(&mut self, context: &egui::Context) {
        let mut open = self.rename_open;
        if self.ui_preferences.floating_windows {
            super::window_frame::native_popup(
                context,
                "native_rename",
                "Rename chat",
                egui::vec2(380.0, 180.0),
                &mut open,
                |ui| {
                    if let Some(chat) = self.session.active_mut() {
                        ui.add(
                            egui::TextEdit::singleline(&mut chat.title)
                                .desired_width(f32::INFINITY),
                        );
                    }
                },
            );
            self.rename_open = open;
            return;
        }
        egui::Window::new("Rename chat")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(context, |ui| {
                if let Some(chat) = self.session.active_mut() {
                    ui.add(egui::TextEdit::singleline(&mut chat.title).desired_width(300.0));
                }
            });
        self.rename_open = open;
    }

    pub(in crate::app) fn approval_dialog(&mut self, context: &egui::Context) {
        let mut decision = None;
        if let Some(approval) = &self.approval {
            let mut body = |ui: &mut egui::Ui| {
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(&approval.description)
                                .wrap()
                                .selectable(true),
                        );
                    });
                ui.horizontal(|ui| {
                    if ui.button("Approve once").clicked() {
                        decision = Some(true);
                    }
                    if ui.button("Reject").clicked() {
                        decision = Some(false);
                    }
                });
            };
            if self.ui_preferences.floating_windows {
                let mut open = true;
                super::window_frame::native_popup(
                    context,
                    "native_approval",
                    "Assistant requests approval",
                    egui::vec2(640.0, 420.0),
                    &mut open,
                    &mut body,
                );
                if !open {
                    decision = Some(false);
                }
            } else {
                egui::Window::new("Assistant requests approval")
                    .collapsible(false)
                    .show(context, body);
            }
        }
        if let Some(decision) = decision
            && let Some(approval) = self.approval.take()
        {
            let _ = approval.response.send(decision);
            self.status = "Generating...".into();
        }
    }
}
