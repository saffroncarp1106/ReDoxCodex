use std::time::{Duration, Instant};

use eframe::egui::{self, Popup, PopupCloseBehavior, Response, SetOpenCommand};
use redox::domain::MessageStatus;

use crate::app::ReDox;

impl ReDox {
    pub(super) fn queue_send_control(&mut self, response: Response) -> bool {
        let response = response.on_hover_text("Send message · Drag up or right-click to queue");
        let dragged_up = response.dragged_by(egui::PointerButton::Primary)
            && response
                .total_drag_delta()
                .is_some_and(|delta| delta.y < -24.0 && -delta.y > delta.x.abs());
        let open = dragged_up || response.secondary_clicked();
        let popup = Popup::menu(&response)
            .open_memory(if open {
                Some(SetOpenCommand::Bool(true))
            } else if response.clicked() {
                Some(SetOpenCommand::Bool(false))
            } else {
                None
            })
            .align(egui::emath::RectAlign::TOP_END)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside);
        let custom_key = response.id.with("queue_delay_minutes");
        let delay = popup
            .show(|ui| {
                ui.strong("Queue message");
                let when_idle = if self.running.is_some() {
                    "After current response"
                } else {
                    "When idle"
                };
                let mut delay = None;
                if ui.button(when_idle).clicked() {
                    delay = Some(Duration::ZERO);
                }
                if ui.button("After 10 minutes").clicked() {
                    delay = Some(Duration::from_secs(600));
                }
                let mut minutes = ui
                    .data(|data| data.get_temp::<u32>(custom_key))
                    .unwrap_or(10);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut minutes)
                            .range(1..=1440)
                            .suffix(" min"),
                    );
                    if ui.button("Queue custom delay").clicked() {
                        delay = Some(Duration::from_secs(u64::from(minutes) * 60));
                    }
                });
                ui.data_mut(|data| data.insert_temp(custom_key, minutes));
                ui.label(egui::RichText::new("Sends when ReDox is open and idle.").small());
                if delay.is_some() {
                    ui.close();
                }
                delay
            })
            .and_then(|result| result.inner);
        if let Some(delay) = delay {
            match self.session.queue_prompt_after(delay) {
                Ok(_) => {
                    self.attachment_error = None;
                    self.focus_prompt = true;
                    response.ctx.request_repaint();
                }
                Err(error) => self.attachment_error = Some(error),
            }
        }
        response.clicked()
    }

    pub(super) fn queued_messages(&mut self, ui: &mut egui::Ui) {
        let Some(chat) = self.session.active_mut() else {
            return;
        };
        let pending = || {
            chat.messages
                .iter()
                .filter(|message| message.status == MessageStatus::Queued)
        };
        let count = pending().count();
        if count == 0 {
            return;
        }
        let mut cancel = None;
        let now = Instant::now();
        ui.label(egui::RichText::new(format!("Queued messages ({count})")).small());
        egui::ScrollArea::vertical()
            .id_salt(("queued_messages", chat.id))
            .max_height(108.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for message in pending() {
                    ui.push_id(message.id, |ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui.small_button("Cancel").clicked() {
                                    cancel = Some(message.id);
                                }
                                let preview = if message.content.is_empty() {
                                    message
                                        .attachments
                                        .first()
                                        .map(|file| file.name())
                                        .unwrap_or_default()
                                } else {
                                    message.content.chars().take(48).collect()
                                };
                                let timing = match chat.send_after.get(&message.id) {
                                    Some(deadline) if *deadline > now => {
                                        let remaining = *deadline - now;
                                        ui.ctx().request_repaint_after(
                                            remaining.min(Duration::from_secs(1)),
                                        );
                                        let seconds = remaining.as_secs()
                                            + u64::from(remaining.subsec_nanos() != 0);
                                        format!(
                                            "{:02}:{:02} until send",
                                            seconds / 60,
                                            seconds % 60
                                        )
                                    }
                                    _ if self.running.is_some() => {
                                        "Waiting for current response".into()
                                    }
                                    _ => "Ready to send".into(),
                                };
                                ui.label(egui::RichText::new(timing).small());
                                ui.add(egui::Label::new(preview).truncate());
                            },
                        );
                    });
                }
            });
        if let Some(id) = cancel {
            chat.cancel_queued(
                id,
                "Queue cancelled. Copy this message to send it again.".into(),
            );
        }
    }
}
