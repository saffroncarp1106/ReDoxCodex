use super::controls::model_label;
use super::icons::{self, Icon};
use super::theme::{self, background, muted, surface, text};
use crate::app::ForgeDesk;
use eframe::egui::{
    self, Align, Color32, FontId, Frame, Layout, Margin, RichText, Sense, Stroke, vec2,
};
use forgedesk::domain::{DEFAULT_MODEL, Message, MessageStatus, Mode, Role};

impl ForgeDesk {
    pub(in crate::app) fn conversation(&mut self, context: &egui::Context) {
        egui::TopBottomPanel::top("conversation_header")
            .exact_height(54.0)
            .show_separator_line(false)
            .frame(
                Frame::NONE
                    .fill(background())
                    .inner_margin(Margin::symmetric(22, 10)),
            )
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(18.0, 22.0), Sense::hover());
                    icons::paint(ui.painter(), rect.shrink(2.0), Icon::Folder, text());
                    ui.add_space(6.0);
                    let title = self
                        .session
                        .active()
                        .map_or("New chat", |chat| chat.title.as_str());
                    ui.allocate_ui_with_layout(
                        vec2(ui.available_width() - 102.0, 30.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            ui.add(
                                egui::Label::new(RichText::new(title).font(theme::semibold(14.0)))
                                    .truncate(),
                            );
                        },
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::button(ui, Icon::Panel, "Toggle sidebar").clicked() {
                            self.sidebar_visible = !self.sidebar_visible;
                        }
                        if icons::button(ui, Icon::Sliders, "Chat settings").clicked() {
                            self.settings_open = true;
                        }
                        ui.menu_button("···", |ui| {
                            if ui.button("Rename chat").clicked() {
                                self.rename_open = true;
                                ui.close();
                            }
                            if let Some(chat) = self.session.active_mut() {
                                let label = if chat.archived {
                                    "Restore chat"
                                } else {
                                    "Archive chat"
                                };
                                if ui.button(label).clicked() {
                                    chat.archived = !chat.archived;
                                    self.show_archived = chat.archived;
                                    ui.close();
                                }
                            }
                        });
                    });
                });
            });

        self.composer(context);

        egui::CentralPanel::default()
            .frame(
                Frame::NONE
                    .fill(background())
                    .inner_margin(Margin::symmetric(26, 0)),
            )
            .show(context, |ui| {
                let Some(chat) = self.session.active() else {
                    return;
                };
                if let Some(texture) = &self.ui_background {
                    ui.painter().image(
                        texture.id(),
                        ui.max_rect(),
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                if chat.messages.is_empty() {
                    ui.add_space((ui.available_height() * 0.35).max(30.0));
                    ui.vertical_centered(|ui| {
                        ui.heading(&self.active_ui.config.heading);
                        ui.add_space(9.0);
                        ui.label(
                            RichText::new("Ask a question or start working on your project.")
                                .color(muted()),
                        );
                    });
                    return;
                }

                egui::ScrollArea::vertical()
                    .id_salt(chat.id)
                    .stick_to_bottom(true)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let content_width = ui.available_width().min(820.0);
                        let margin = (ui.available_width() - content_width) * 0.5;
                        ui.horizontal_top(|ui| {
                            ui.add_space(margin);
                            ui.vertical(|ui| {
                                ui.set_width(content_width);
                                ui.add_space(10.0);
                                for message in &chat.messages {
                                    render_message(ui, message);
                                    ui.add_space(26.0);
                                }
                                ui.add_space(16.0);
                            });
                        });
                    });
            });
    }

    fn composer(&mut self, context: &egui::Context) {
        let mut send = false;
        let mut stop = false;
        egui::TopBottomPanel::bottom("composer")
            .show_separator_line(false)
            .frame(Frame::NONE.fill(background()).inner_margin(Margin {
                left: 16,
                right: 10,
                top: 12,
                bottom: 18,
            }))
            .show(context, |ui| {
                Frame::new()
                    .fill(surface())
                    .stroke(Stroke::new(1.0_f32, Color32::from_gray(59)))
                    .corner_radius(20)
                    .inner_margin(Margin::symmetric(12, 11))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.set_min_width(ui.available_width());
                        if let Some(chat) = self.session.active_mut() {
                            let mut remove = None;
                            ui.horizontal_wrapped(|ui| {
                                for file in &chat.attachments {
                                    Frame::new()
                                        .fill(Color32::from_gray(62))
                                        .corner_radius(8)
                                        .inner_margin(6)
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new(file.name()).size(12.0))
                                                    .on_hover_text(file.path.display().to_string());
                                                if ui
                                                    .small_button("×")
                                                    .on_hover_text("Remove attachment")
                                                    .clicked()
                                                {
                                                    remove = Some(file.id);
                                                }
                                            });
                                        });
                                }
                            });
                            if let Some(id) = remove {
                                chat.attachments.retain(|file| file.id != id);
                            }
                        }
                        if self.picking_files {
                            ui.label(RichText::new("Loading files...").size(12.0).color(muted()));
                        }
                        if let Some(error) = &self.attachment_error {
                            ui.label(
                                RichText::new(error)
                                    .size(12.0)
                                    .color(Color32::from_rgb(235, 150, 135)),
                            );
                        }
                        if let Some(chat) = self.session.active_mut() {
                            ui.add_enabled_ui(!chat.archived, |ui| {
                                let response = ui.add(
                                    egui::TextEdit::multiline(&mut chat.draft)
                                        .id_salt("prompt_input")
                                        .frame(false)
                                        .font(FontId::proportional(14.0))
                                        .hint_text(&self.active_ui.config.placeholder)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(2)
                                        .return_key(egui::KeyboardShortcut::new(
                                            egui::Modifiers::SHIFT,
                                            egui::Key::Enter,
                                        )),
                                );
                                if response.has_focus()
                                    && context.input(|input| {
                                        input.key_pressed(egui::Key::Enter)
                                            && !input.modifiers.shift
                                            && !input
                                                .events
                                                .iter()
                                                .any(|event| matches!(event, egui::Event::Ime(_)))
                                    })
                                {
                                    send = true;
                                }
                            });
                        }
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.menu_button(RichText::new("+").size(23.0), |ui| {
                                if ui
                                    .add_enabled(
                                        !self.picking_files,
                                        egui::Button::new("Add files"),
                                    )
                                    .clicked()
                                {
                                    self.pick_files(context);
                                    ui.close();
                                }
                                if ui.button("Open project folder").clicked() {
                                    self.pick_workspace(context);
                                    ui.close();
                                }
                            });
                            if let Some(chat) = self.session.active_mut() {
                                let editable = self.running.is_none();
                                let color = if chat.mode != Mode::Chat {
                                    Color32::from_rgb(231, 120, 49)
                                } else {
                                    muted()
                                };
                                let (rect, _) =
                                    ui.allocate_exact_size(vec2(15.0, 18.0), Sense::hover());
                                icons::paint(ui.painter(), rect.shrink(1.0), Icon::Shield, color);
                                let mut mode = chat.mode;
                                ui.add_enabled_ui(editable, |ui| {
                                    ui.menu_button(
                                        RichText::new(chat.mode.label()).size(12.0).color(color),
                                        |ui| {
                                            for option in [Mode::Chat, Mode::Code, Mode::FullAccess]
                                            {
                                                if ui
                                                    .selectable_value(
                                                        &mut mode,
                                                        option,
                                                        option.label(),
                                                    )
                                                    .clicked()
                                                {
                                                    ui.close();
                                                }
                                            }
                                        },
                                    );
                                });
                                chat.set_mode(mode);
                            }

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let can_send = self.runtime.is_some()
                                    && !self.picking_files
                                    && self.session.active().is_some_and(|chat| {
                                        !chat.archived
                                            && (!chat.draft.trim().is_empty()
                                                || !chat.attachments.is_empty())
                                    });
                                let running = self.running.is_some();
                                let (rect, response) =
                                    ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
                                let fill = if can_send || running {
                                    theme::blue()
                                } else {
                                    Color32::from_gray(85)
                                };
                                ui.painter().circle_filled(rect.center(), 14.0, fill);
                                if running {
                                    ui.painter().rect_filled(
                                        egui::Rect::from_center_size(rect.center(), vec2(9.0, 9.0)),
                                        1,
                                        text(),
                                    );
                                } else {
                                    icons::paint(
                                        ui.painter(),
                                        rect.shrink(6.0),
                                        Icon::ArrowUp,
                                        text(),
                                    );
                                }
                                if response
                                    .on_hover_text(if running {
                                        "Stop response"
                                    } else {
                                        "Send message"
                                    })
                                    .clicked()
                                {
                                    if running {
                                        stop = true;
                                    } else if can_send {
                                        send = true;
                                    }
                                }
                                ui.add_enabled_ui(false, |ui| {
                                    icons::button(
                                        ui,
                                        Icon::Mic,
                                        "Voice input is not available yet",
                                    );
                                });
                                self.effort_picker(ui, "composer_effort");
                                egui::ComboBox::from_id_salt("composer_model")
                                    .selected_text(
                                        RichText::new(model_label(&self.session.model)).size(12.0),
                                    )
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut self.session.model,
                                            DEFAULT_MODEL.into(),
                                            DEFAULT_MODEL,
                                        );
                                        for model in &self.models {
                                            ui.selectable_value(
                                                &mut self.session.model,
                                                model.clone(),
                                                model_label(model),
                                            );
                                        }
                                    });
                            });
                        });
                    });
                if self.running.is_some() {
                    ui.add(
                        egui::Label::new(RichText::new(&self.status).size(12.0).color(muted()))
                            .truncate(),
                    );
                }
                if self.models.is_empty() && !self.refreshing && !self.status.starts_with("Ready") {
                    ui.label(
                        RichText::new(&self.status)
                            .size(12.0)
                            .color(Color32::from_rgb(225, 145, 130)),
                    );
                }
            });

        if send {
            self.send_prompt(context);
        }
        if stop {
            self.stop_response();
        }
    }
}

pub(super) fn render_message(ui: &mut egui::Ui, message: &Message) {
    if message.role == Role::User {
        ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
            Frame::new()
                .fill(Color32::from_rgb(25, 62, 116))
                .corner_radius(19)
                .inner_margin(Margin::symmetric(15, 10))
                .show(ui, |ui| {
                    ui.set_max_width((ui.available_width() * 0.72).max(180.0));
                    for file in &message.attachments {
                        ui.label(
                            RichText::new(format!("▧ {}", file.name()))
                                .size(12.0)
                                .color(Color32::from_gray(215)),
                        );
                    }
                    ui.add(
                        egui::Label::new(paragraph(&message.content))
                            .wrap()
                            .selectable(true),
                    );
                });
        });
        return;
    }

    if message.content.is_empty() && message.status == MessageStatus::Streaming {
        ui.horizontal(|ui| {
            ui.add(egui::Spinner::new().size(14.0));
            ui.label(RichText::new("Working...").color(muted()));
        });
    } else {
        render_markdown(ui, &message.content);
    }

    if let Some(error) = &message.error {
        ui.label(RichText::new(error).color(Color32::from_rgb(235, 150, 135)));
    }
    if matches!(
        message.status,
        MessageStatus::Cancelled | MessageStatus::Interrupted
    ) {
        ui.label(RichText::new("Response stopped").size(12.0).color(muted()));
    }
    if message.status != MessageStatus::Streaming && !message.content.is_empty() {
        ui.add_space(10.0);
        if icons::button(ui, Icon::Copy, "Copy response").clicked() {
            ui.ctx().copy_text(message.content.clone());
        }
    }
}

fn paragraph(text: &str) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    for (index, part) in text.split("**").enumerate() {
        job.append(
            part,
            0.0,
            egui::TextFormat {
                font_id: if index % 2 == 1 {
                    theme::semibold(14.0)
                } else {
                    FontId::proportional(14.0)
                },
                color: theme::text(),
                line_height: Some(23.0),
                ..Default::default()
            },
        );
    }
    job
}

fn render_markdown(ui: &mut egui::Ui, text: &str) {
    for (index, section) in text.split("```").enumerate() {
        if index % 2 == 0 {
            if !section.trim().is_empty() {
                ui.add(
                    egui::Label::new(paragraph(section.trim()))
                        .wrap()
                        .selectable(true),
                );
            }
        } else {
            let (language, code) = section.split_once('\n').unwrap_or(("", section));
            ui.add_space(8.0);
            Frame::new()
                .fill(Color32::from_gray(33))
                .corner_radius(10)
                .inner_margin(12)
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(language).size(12.0).color(muted()));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if icons::button(ui, Icon::Copy, "Copy code").clicked() {
                                ui.ctx().copy_text(code.trim_end().into());
                            }
                        });
                    });
                    ui.add(
                        egui::Label::new(RichText::new(code.trim_end()).monospace())
                            .wrap()
                            .selectable(true),
                    );
                });
            ui.add_space(8.0);
        }
    }
}
