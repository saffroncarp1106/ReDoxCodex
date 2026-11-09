use crate::app::ReDox;
use eframe::egui;

impl ReDox {
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
            let mut open = true;
            let mut body = |ui: &mut egui::Ui| {
                ui.label(egui::RichText::new(approval.summary).strong());
                let details = if self.approval_details_open {
                    "▾ Details"
                } else {
                    "▸ Details"
                };
                if ui.small_button(details).clicked() {
                    self.approval_details_open = !self.approval_details_open;
                }
                if self.approval_details_open {
                    super::scroll::ScrollArea::vertical()
                        .max_height(160.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(&approval.description)
                                    .wrap()
                                    .selectable(true),
                            );
                        });
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Approve").color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::from_rgb(39, 93, 188)),
                        )
                        .clicked()
                    {
                        decision = Some(true);
                    }
                    if ui.button("Reject").clicked() {
                        decision = Some(false);
                    }
                });
            };
            if self.ui_preferences.floating_windows {
                super::window_frame::native_popup(
                    context,
                    "native_approval",
                    "Assistant requests approval",
                    egui::vec2(440.0, 300.0),
                    &mut open,
                    &mut body,
                );
            } else {
                egui::Window::new("Assistant requests approval")
                    .open(&mut open)
                    .default_width(420.0)
                    .collapsible(false)
                    .show(context, body);
            }
            if !open {
                decision = Some(false);
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
