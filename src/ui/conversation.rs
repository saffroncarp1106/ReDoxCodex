use super::icons::{self, Icon};
use super::theme::{self, background, muted, text};
use crate::app::ReDox;
use eframe::egui::{
    self, Align, Color32, FontId, Frame, Layout, Margin, RichText, Sense, Stroke, vec2,
};
use redox::domain::{Chat, Message, MessageStatus, Mode, Role};

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
        self.composer(context, None);

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
                        let (rect, _) = ui.allocate_exact_size(vec2(52.0, 52.0), Sense::hover());
                        icons::paint(ui.painter(), rect, Icon::Codex, Color32::from_gray(100));
                        ui.add_space(24.0);
                        let heading = chat
                            .workspace
                            .as_ref()
                            .and_then(|path| path.file_name())
                            .map(|name| {
                                format!("What should we build in {}?", name.to_string_lossy())
                            })
                            .unwrap_or_else(|| "What should we build?".into());
                        ui.label(RichText::new(heading).size(28.0));
                        ui.add_space(9.0);
                        ui.label(
                            RichText::new("Ask a question or start working on your project.")
                                .color(muted()),
                        );
                    });
                    return;
                }

                render_transcript(ui, chat, chat.id, 0.0, self.ui_preferences.codex_mode);
            });
    }

    pub(super) fn composer(&mut self, context: &egui::Context, parent: Option<&mut egui::Ui>) {
        let classic = parent.is_some();
        let mut send = false;
        let mut stop = false;
        let panel =
            egui::TopBottomPanel::bottom(egui::Id::new(("composer", context.viewport_id())))
                .show_separator_line(false)
                .frame(if classic {
                    Frame::NONE.fill(theme::surface()).inner_margin(8)
                } else {
                    Frame::NONE.fill(background()).inner_margin(Margin {
                        left: 16,
                        right: 10,
                        top: 12,
                        bottom: 18,
                    })
                });
        let contents = |ui: &mut egui::Ui| {
            ui.set_clip_rect(ui.clip_rect().intersect(ui.max_rect()));
            Frame::new()
                .fill(theme::input_background())
                .stroke(Stroke::new(
                    1.0,
                    if classic {
                        theme::border()
                    } else {
                        Color32::from_gray(59)
                    },
                ))
                .corner_radius(if classic { 0 } else { 20 })
                .inner_margin(Margin::symmetric(12, 6))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.set_max_width(ui.available_width());
                    if let Some(chat) = self.session.active_mut() {
                        let mut remove = None;
                        let available = ui.available_width();
                        let columns = ((available + 5.0) / 257.0).floor().max(1.0) as usize;
                        let chip_width =
                            ((available - (columns - 1) as f32 * 5.0) / columns as f32 - 12.0)
                                .clamp(1.0, 240.0);
                        ui.vertical(|ui| {
                            for row in chat.attachments.chunks(columns) {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 5.0;
                                    for file in row {
                                        Frame::new()
                                            .fill(if classic {
                                                theme::surface()
                                            } else {
                                                Color32::from_gray(62)
                                            })
                                            .corner_radius(if classic { 0 } else { 8 })
                                            .inner_margin(6)
                                            .show(ui, |ui| {
                                                ui.set_width(chip_width);
                                                ui.horizontal(|ui| {
                                                    ui.add_sized(
                                                        [(chip_width - 33.0).max(1.0), 20.0],
                                                        egui::Label::new(
                                                            RichText::new(file.name()).size(12.0),
                                                        )
                                                        .truncate(),
                                                    )
                                                    .on_hover_ui(|ui| {
                                                        ui.label(file.path.display().to_string());
                                                        if file.text.is_none() {
                                                            ui.set_max_width(280.0);
                                                            super::media::show(
                                                                ui,
                                                                &file.path.to_string_lossy(),
                                                                false,
                                                                None,
                                                            );
                                                        }
                                                    });
                                                    if ui
                                                        .small_button("\u{00d7}")
                                                        .on_hover_text("Remove attachment")
                                                        .clicked()
                                                    {
                                                        remove = Some(file.id);
                                                    }
                                                });
                                            });
                                    }
                                });
                            }
                        });
                        if let Some(id) = remove {
                            chat.attachments.retain(|file| file.id != id);
                        }
                    }
                    if self
                        .session
                        .active()
                        .is_some_and(|chat| !chat.attachments.is_empty())
                    {
                        ui.add_space(10.0);
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
                                    .desired_width(ui.available_width())
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
                    ui.allocate_ui_with_layout(
                        vec2(ui.available_width(), 34.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = 3.0;
                            ui.spacing_mut().button_padding.x = 5.0;
                            if classic {
                                ui.spacing_mut().interact_size.y = 28.0;
                                ui.spacing_mut().button_padding.y = 0.0;
                            }
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
                                ui.add_space(5.0);
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
                                let sense = if can_send {
                                    Sense::click_and_drag()
                                } else {
                                    Sense::click()
                                };
                                if classic {
                                    let response = ui.add_enabled(
                                        can_send,
                                        egui::Button::new("Send")
                                            .min_size(vec2(58.0, 28.0))
                                            .sense(sense),
                                    );
                                    super::win98::bevel(
                                        ui.painter(),
                                        response.rect,
                                        response.is_pointer_button_down_on(),
                                    );
                                    if can_send {
                                        send |= self.queue_send_control(response);
                                    }
                                    if running {
                                        stop |= super::win98::button(ui, "Stop", vec2(52.0, 28.0))
                                            .clicked();
                                    }
                                    self.model_effort_picker(ui, "composer_model_effort");
                                    return;
                                }
                                let (rect, response) =
                                    ui.allocate_exact_size(vec2(28.0, 28.0), sense);
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
                                if can_send {
                                    send |= self.queue_send_control(response);
                                } else if running {
                                    stop |= response.on_hover_text("Stop response").clicked();
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
                        },
                    );
                });
            self.queued_messages(ui);
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
        };
        if let Some(ui) = parent {
            panel.show_inside(ui, contents);
        } else {
            panel.show(context, contents);
        }

        if send {
            self.send_prompt(context);
        }
        if stop {
            self.stop_response();
        }
    }
}

pub(super) fn render_transcript(
    ui: &mut egui::Ui,
    chat: &Chat,
    scroll_id: impl std::hash::Hash,
    horizontal_padding: f32,
    codex_mode: bool,
) {
    ui.spacing_mut().scroll = theme::conversation_scroll();
    super::scroll::ScrollArea::vertical()
        .id_salt(scroll_id)
        .horizontal_scroll_offset(0.0)
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let available = ui.available_rect_before_wrap();
            let width = (available.width() - horizontal_padding)
                .clamp(260.0, 820.0)
                .min(available.width());
            let left = available.center().x - width * 0.5;
            let right = left + width;
            ui.add_space(16.0);
            for message in &chat.messages {
                let top = ui.next_widget_position().y;
                let bounds = egui::Rect::from_min_max(
                    egui::pos2(left, top),
                    egui::pos2(right, available.bottom().max(top)),
                );
                let mut row = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt(message.id)
                        .max_rect(bounds)
                        .layout(Layout::top_down(Align::Min)),
                );
                let clip = ui.clip_rect();
                row.set_clip_rect(clip.intersect(egui::Rect::from_min_max(
                    egui::pos2(left, clip.top()),
                    egui::pos2(right, clip.bottom()),
                )));
                render_message(&mut row, message, chat.workspace.as_deref(), codex_mode);
                // An oversized message must not widen the layout used for the next row.
                ui.advance_cursor_after_rect(egui::Rect::from_min_max(
                    bounds.min,
                    egui::pos2(right, row.min_rect().bottom()),
                ));
                ui.add_space(26.0);
            }
            ui.add_space(16.0);
        });
}

fn render_message(
    ui: &mut egui::Ui,
    message: &Message,
    workspace: Option<&std::path::Path>,
    codex_mode: bool,
) {
    if message.role == Role::User {
        let classic = !ui.visuals().dark_mode;
        let bubble_width = ui.available_width() * 0.72;
        ui.with_layout(Layout::top_down(Align::Max), |ui| {
            Frame::new()
                .fill(if classic {
                    Color32::from_rgb(218, 231, 247)
                } else {
                    Color32::from_rgb(25, 62, 116)
                })
                .corner_radius(if classic { 0 } else { 19 })
                .inner_margin(Margin::symmetric(15, 10))
                .show(ui, |ui| {
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        ui.set_max_width(bubble_width.min(ui.available_width()));
                        for file in &message.attachments {
                            if file.text.is_none() {
                                super::media::show(
                                    ui,
                                    &file.path.to_string_lossy(),
                                    false,
                                    workspace,
                                );
                            }
                            ui.label(RichText::new(format!("{}", file.name())).size(12.0).color(
                                if classic {
                                    muted()
                                } else {
                                    Color32::from_gray(215)
                                },
                            ));
                        }
                        if !message.attachments.is_empty() {
                            ui.add_space(12.0);
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
        });
        return;
    }

    let show_activity = codex_mode && !message.activity.is_empty();
    if message.content.is_empty() && message.status == MessageStatus::Streaming && !show_activity {
        ui.horizontal(|ui| {
            ui.add(egui::Spinner::new().size(14.0));
            ui.label(RichText::new("Working...").color(muted()));
        });
    } else {
        if message.model.starts_with("Ollama ·") {
            ui.label(RichText::new(&message.model).small().color(muted()));
        }
        ui.push_id(message.id, |ui| {
            if show_activity {
                let mut start = 0;
                for (index, command) in message.activity.commands.iter().enumerate() {
                    if let Some(text) = message.content.get(start..command.offset) {
                        if !text.is_empty() {
                            ui.push_id(index, |ui| {
                                super::response_text::show_text(ui, message.id, text, workspace);
                            });
                        }
                        start = command.offset;
                    }
                    super::activity::command(
                        ui,
                        command,
                        message.status == MessageStatus::Streaming,
                    );
                }
                if start < message.content.len() {
                    super::response_text::show_text(
                        ui,
                        message.id,
                        &message.content[start..],
                        workspace,
                    );
                }
                super::activity::changes(ui, &message.activity);
            } else {
                super::response_text::show(ui, message, workspace);
            }
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

#[cfg(test)]
#[path = "../../tests/ui/transcript_layout.rs"]
mod layout_tests;
