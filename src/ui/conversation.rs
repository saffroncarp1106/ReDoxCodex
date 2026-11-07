use super::icons::{self, Icon};
use super::theme::{self, background, muted, text};
use crate::app::ReDox;
use eframe::egui::{
    self, Align, Color32, FontId, Frame, Layout, Margin, RichText, Sense, Stroke, vec2,
};
use redox::domain::{Message, MessageStatus, Mode, Role};

impl ReDox {
    pub(in crate::app) fn conversation(&mut self, context: &egui::Context) {
        if context.viewport_id() == egui::ViewportId::ROOT {
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
                        if icons::button(ui, Icon::Panel, "Toggle sidebar").clicked() {
                            self.sidebar_visible = !self.sidebar_visible;
                        }
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
                                    egui::Label::new(
                                        RichText::new(title).font(theme::semibold(14.0)),
                                    )
                                    .truncate(),
                                );
                            },
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if icons::button(ui, Icon::Agent, "Toggle Ollama agents").clicked() {
                                self.ui_preferences.agents_open = !self.ui_preferences.agents_open;
                                context.request_repaint();
                            }
                            ui.menu_button("···", |ui| {
                                if ui.button("Rename chat").clicked() {
                                    self.rename_open = true;
                                    ui.close();
                                }
                                if let Some(chat) = self.session.active() {
                                    let id = chat.id;
                                    let archived = chat.archived;
                                    let label = if chat.archived {
                                        "Restore chat"
                                    } else {
                                        "Archive chat"
                                    };
                                    if ui.button(label).clicked() {
                                        self.archive_local_chat(id, !archived);
                                        ui.close();
                                    }
                                }
                            });
                        });
                    });
                });
        }
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
                    super::skins::paint_background(
                        ui.painter(),
                        ui.max_rect(),
                        texture,
                        self.active_ui.config.background_opacity,
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

                ui.spacing_mut().scroll = theme::conversation_scroll();
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
                                    render_message(ui, message, chat.workspace.as_deref());
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
                    .fill(theme::input_background())
                    .stroke(Stroke::new(1.0_f32, Color32::from_gray(59)))
                    .corner_radius(20)
                    .inner_margin(Margin::symmetric(12, 6))
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
                                        .desired_rows(1)
                                        .return_key(egui::KeyboardShortcut::new(
                                            egui::Modifiers::SHIFT,
                                            egui::Key::Enter,
                                        )),
                                );
                                self.prompt_input_id = Some(response.id);
                                super::controls::focus_new_prompt(
                                    &response,
                                    &mut self.focus_prompt,
                                    !self.startup.is_active()
                                        && !self.settings_open
                                        && !self.rename_open,
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
                            ui.spacing_mut().item_spacing.x = 3.0;
                            ui.spacing_mut().button_padding.x = 5.0;
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
                                if context.viewport_id() == egui::ViewportId::ROOT
                                    && ui.button("Open project folder").clicked()
                                {
                                    self.pick_workspace(context);
                                    ui.close();
                                }
                            });
                            if let Some(chat) = self.session.active_mut()
                                && chat.workspace.is_some()
                            {
                                let editable = self.running.is_none();
                                let color = if chat.mode == Mode::FullAccess {
                                    Color32::from_rgb(231, 120, 49)
                                } else {
                                    text()
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
                            } else {
                                ui.label(RichText::new("Chat").size(12.0))
                                    .on_hover_text("No project folder");
                            }

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let can_send = self.runtime.is_some()
                                    && !self.picking_files
                                    && self.session.active().is_some_and(|chat| {
                                        !chat.archived
                                            && (!chat.draft.trim().is_empty()
                                                || !chat.attachments.is_empty())
                                    });
                                let running = self.active_chat_running();
                                let (rect, response) =
                                    ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
                                let fill = if can_send || running {
                                    theme::blue()
                                } else {
                                    Color32::from_gray(85)
                                };
                                ui.painter().circle_filled(rect.center(), 14.0, fill);
                                if running && !can_send {
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
                                    .on_hover_text(if running && !can_send {
                                        "Stop response"
                                    } else {
                                        "Send message"
                                    })
                                    .clicked()
                                {
                                    if running && !can_send {
                                        stop = true;
                                    } else if can_send {
                                        send = true;
                                    }
                                }
                                if running
                                    && can_send
                                    && icons::button(ui, Icon::Close, "Stop response").clicked()
                                {
                                    stop = true;
                                }
                                ui.add_enabled_ui(false, |ui| {
                                    icons::button(
                                        ui,
                                        Icon::Mic,
                                        "Voice input is not available yet",
                                    );
                                });
                                self.model_effort_picker(ui, "composer_model_effort");
                            });
                        });
                    });
                if self.active_chat_running() {
                    ui.add(
                        egui::Label::new(RichText::new(&self.status).size(12.0).color(muted()))
                            .truncate(),
                    );
                }
                if !self.is_ollama()
                    && self.models.is_empty()
                    && !self.refreshing
                    && !self.status.starts_with("Ready")
                {
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

pub(super) fn render_message(
    ui: &mut egui::Ui,
    message: &Message,
    workspace: Option<&std::path::Path>,
) {
    if message.role == Role::User {
        ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
            Frame::new()
                .fill(Color32::from_rgb(25, 62, 116))
                .corner_radius(19)
                .inner_margin(Margin::symmetric(15, 10))
                .show(ui, |ui| {
                    ui.set_max_width((ui.available_width() * 0.72).max(180.0));
                    for file in &message.attachments {
                        if file.text.is_none() {
                            super::media::show(ui, &file.path.to_string_lossy(), false, workspace);
                        }
                        ui.label(
                            RichText::new(format!("▧ {}", file.name()))
                                .size(12.0)
                                .color(Color32::from_gray(215)),
                        );
                    }
                    ui.push_id(message.id, |ui| {
                        super::markdown::show_in_workspace(ui, &message.content, workspace)
                    });
                    if message.status == MessageStatus::Queued && message.error.is_none() {
                        ui.label(RichText::new("Pending").small());
                    }
                    if let Some(error) = &message.error {
                        ui.label(RichText::new(error).small());
                    }
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
        if message.model.starts_with("Ollama ·") {
            ui.label(RichText::new(&message.model).small().color(muted()));
        }
        ui.push_id(message.id, |ui| {
            super::response_text::show(ui, message, workspace);
        });
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
