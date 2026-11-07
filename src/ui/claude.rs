use eframe::egui::{
    self, Align, Color32, FontId, Frame, Layout, Margin, Rect, RichText, Sense, Stroke, pos2, vec2,
};
use redox::domain::Mode;

use super::{
    icons::{self, Icon},
    theme,
};
use crate::app::ReDox;

impl ReDox {
    pub(in crate::app) fn claude_rail(&mut self, context: &egui::Context) {
        egui::SidePanel::left("claude_rail")
            .exact_width(64.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                Frame::NONE
                    .fill(theme::rail())
                    .inner_margin(Margin::symmetric(14, 14)),
            )
            .show(context, |ui| {
                ui.spacing_mut().item_spacing.y = 14.0;
                for (icon, label) in [
                    (Icon::NewChat, "New chat"),
                    (Icon::Search, "Search chats"),
                    (Icon::Grid, "Projects and chats"),
                    (Icon::Books, "Open workspace"),
                    (Icon::Clock, "Archived chats"),
                    (Icon::Shield, "Workspace access"),
                ] {
                    let enabled = !matches!(icon, Icon::NewChat) || self.can_new_chat();
                    if ui
                        .add_enabled_ui(enabled, |ui| icons::button(ui, icon, label))
                        .inner
                        .clicked()
                    {
                        match icon {
                            Icon::NewChat => {
                                self.new_chat();
                            }
                            Icon::Search => {
                                self.sidebar_visible = true;
                                self.search_open = true;
                            }
                            Icon::Grid => self.sidebar_visible = !self.sidebar_visible,
                            Icon::Books => self.pick_workspace(context),
                            Icon::Clock => {
                                self.show_archived = !self.show_archived;
                                self.sidebar_visible = true;
                            }
                            _ => {
                                self.settings_open = true;
                                self.settings_page = super::SettingsPage::Workspace;
                            }
                        }
                    }
                }
                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    if icons::button(ui, Icon::Home, "Account").clicked() {
                        self.settings_open = true;
                        self.settings_page = super::SettingsPage::Account;
                    }
                    if icons::button(ui, Icon::Settings, "Settings").clicked() {
                        self.settings_open = true;
                    }
                    if icons::button(ui, Icon::Reticle, "Usage limits").clicked() {
                        if self.usage_open {
                            self.usage_open = false;
                        } else {
                            self.open_usage(context);
                        }
                    }
                });
            });
    }

    pub(in crate::app) fn claude_conversation(&mut self, context: &egui::Context) {
        let empty = self
            .session
            .active()
            .is_none_or(|chat| chat.messages.is_empty());
        if context.viewport_id() == egui::ViewportId::ROOT {
            egui::TopBottomPanel::top("claude_chat_title")
                .exact_height(44.0)
                .show_separator_line(false)
                .frame(
                    Frame::NONE
                        .fill(theme::background())
                        .inner_margin(Margin::symmetric(24, 7)),
                )
                .show(context, |ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::button(ui, Icon::Agent, "Toggle Ollama agents").clicked() {
                            self.ui_preferences.agents_open = !self.ui_preferences.agents_open;
                            context.request_repaint();
                        }
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            if !empty {
                                let title = self
                                    .session
                                    .active()
                                    .map_or("New chat", |chat| chat.title.as_str());
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(title).size(15.0).color(theme::muted()),
                                    )
                                    .truncate(),
                                );
                            }
                        });
                    });
                });
        }
        if !empty {
            egui::TopBottomPanel::bottom("claude_composer")
                .show_separator_line(false)
                .frame(
                    Frame::NONE
                        .fill(theme::background())
                        .inner_margin(Margin::symmetric(24, 18)),
                )
                .show(context, |ui| {
                    self.claude_centered_composer(ui, context);
                });
        }
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(theme::background()).inner_margin(0))
            .show(context, |ui| {
                let rect = ui.max_rect();
                if let Some(texture) = &self.ui_background {
                    super::skins::paint_background(
                        ui.painter(),
                        ui.max_rect(),
                        texture,
                        self.active_ui.config.background_opacity,
                    );
                }
                if empty {
                    let top = (rect.height() * 0.32 - 50.0).max(28.0);
                    ui.add_space(top);
                    ui.vertical_centered(|ui| {
                        claude_heading(ui, &self.active_ui.config.heading);
                        ui.add_space(20.0);
                    });
                    self.claude_centered_composer(ui, context);
                } else {
                    ui.spacing_mut().scroll = theme::conversation_scroll();
                    egui::ScrollArea::vertical()
                        .id_salt(("claude_chat", self.session.active_chat))
                        .stick_to_bottom(true)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let width = (ui.available_width() - 48.0).clamp(260.0, 820.0);
                            let offset = (ui.available_width() - width) * 0.5;
                            ui.horizontal_top(|ui| {
                                ui.add_space(offset);
                                ui.vertical(|ui| {
                                    ui.set_width(width);
                                    ui.add_space(20.0);
                                    if let Some(chat) = self.session.active() {
                                        for message in &chat.messages {
                                            super::conversation::render_message(
                                                ui,
                                                message,
                                                chat.workspace.as_deref(),
                                            );
                                            ui.add_space(26.0);
                                        }
                                    }
                                });
                            });
                        });
                }
            });
    }

    fn claude_centered_composer(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        let empty = self
            .session
            .active()
            .is_none_or(|chat| chat.messages.is_empty());
        let width = (ui.available_width() - 48.0).clamp(240.0, if empty { 656.0 } else { 768.0 });
        let offset = (ui.available_width() - width) * 0.5;
        ui.horizontal_top(|ui| {
            ui.add_space(offset);
            ui.vertical(|ui| {
                ui.set_width(width);
                self.claude_input(ui, context);
            });
        });
    }

    fn claude_input(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        let empty = self
            .session
            .active()
            .is_none_or(|chat| chat.messages.is_empty());
        let mut send = false;
        if let Some(chat) = self.session.active_mut() {
            let mut remove = None;
            if !chat.attachments.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    for attachment in &chat.attachments {
                        if ui.button(format!("{} ×", attachment.name())).clicked() {
                            remove = Some(attachment.id);
                        }
                    }
                });
            }
            if let Some(id) = remove {
                chat.attachments.retain(|attachment| attachment.id != id);
            }
        }
        composer_box(ui, empty, |ui| {
            if empty {
                send |= self.claude_draft(ui, context, true);
                ui.horizontal(|ui| {
                    ui.spacing_mut().button_padding.y = 2.0;
                    self.claude_add_menu(ui, context);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if self.active_chat_running()
                            || self.session.active().is_some_and(|chat| {
                                !chat.draft.trim().is_empty() || !chat.attachments.is_empty()
                            })
                        {
                            send |= self.claude_send_control(ui);
                        }
                        self.claude_voice_controls(ui, true);
                    });
                });
            } else {
                ui.horizontal_top(|ui| {
                    let controls_width = if self.active_chat_running() {
                        64.0
                    } else {
                        34.0
                    };
                    ui.allocate_ui_with_layout(
                        vec2((ui.available_width() - controls_width).max(100.0), 22.0),
                        Layout::top_down(Align::Min),
                        |ui| send |= self.claude_draft(ui, context, false),
                    );
                    send |= self.claude_send_control(ui);
                });
            }
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if empty && context.viewport_id() == egui::ViewportId::ROOT {
                let project = ui.menu_button(RichText::new("Project     ").size(13.0), |ui| {
                    if let Some(workspace) = &self.session.workspace {
                        ui.label(workspace.display().to_string());
                        ui.separator();
                    }
                    if ui.button("Open project folder…").clicked() {
                        self.pick_workspace(context);
                        ui.close();
                    }
                });
                icons::paint(
                    ui.painter(),
                    Rect::from_center_size(
                        pos2(
                            project.response.rect.right() - 12.0,
                            project.response.rect.center().y,
                        ),
                        vec2(10.0, 10.0),
                    ),
                    Icon::Chevron,
                    theme::muted(),
                );
            } else {
                self.claude_add_menu(ui, context);
                self.claude_voice_controls(ui, true);
            }
            let spare = (ui.available_width() - 284.0).max(0.0);
            let (note, _) = ui.allocate_exact_size(vec2(spare, 28.0), Sense::hover());
            if !empty && spare >= 180.0 {
                ui.painter().text(
                    note.center(),
                    egui::Align2::CENTER_CENTER,
                    "AI can make mistakes.",
                    FontId::proportional(11.0),
                    theme::muted(),
                );
            }
            self.model_effort_picker(ui, "claude_model_effort");
            if let Some(chat) = self.session.active_mut()
                && chat.workspace.is_some()
            {
                let mut mode = chat.mode;
                ui.add_enabled_ui(self.running.is_none(), |ui| {
                    egui::ComboBox::from_id_salt("claude_access")
                        .width(108.0)
                        .selected_text(RichText::new(mode.label()).size(12.0))
                        .show_ui(ui, |ui| {
                            for choice in [Mode::Chat, Mode::Code, Mode::FullAccess] {
                                ui.selectable_value(&mut mode, choice, choice.label());
                            }
                        })
                        .response
                        .on_hover_text("Access for the next message");
                });
                chat.set_mode(mode);
            } else {
                ui.label(RichText::new("Chat").size(12.0))
                    .on_hover_text("No project folder");
            }
        });
        if let Some(error) = &self.attachment_error {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
        if self.picking_files {
            ui.label(
                RichText::new("Loading files…")
                    .small()
                    .color(theme::muted()),
            );
        }
        if self.active_chat_running() {
            ui.label(RichText::new(&self.status).small().color(theme::muted()));
        }
        if send {
            self.send_prompt(context);
        }
    }

    fn claude_draft(&mut self, ui: &mut egui::Ui, context: &egui::Context, empty: bool) -> bool {
        let Some(chat) = self.session.active_mut() else {
            return false;
        };
        let response = ui
            .add_enabled_ui(!chat.archived, |ui| {
                ui.add_sized(
                    [ui.available_width(), if empty { 44.0 } else { 22.0 }],
                    egui::TextEdit::multiline(&mut chat.draft)
                        .id_salt("claude_prompt")
                        .frame(false)
                        .font(FontId::proportional(15.0))
                        .hint_text(
                            RichText::new(if empty {
                                &self.active_ui.config.placeholder
                            } else {
                                "Reply"
                            })
                            .color(theme::muted()),
                        )
                        .desired_rows(if empty { 2 } else { 1 })
                        .return_key(egui::KeyboardShortcut::new(
                            egui::Modifiers::SHIFT,
                            egui::Key::Enter,
                        )),
                )
            })
            .inner;
        self.prompt_input_id = Some(response.id);
        super::controls::focus_new_prompt(
            &response,
            &mut self.focus_prompt,
            !self.startup.is_active() && !self.settings_open && !self.rename_open,
        );
        response.has_focus()
            && context.input(|input| {
                input.key_pressed(egui::Key::Enter)
                    && !input.modifiers.shift
                    && !input
                        .events
                        .iter()
                        .any(|event| matches!(event, egui::Event::Ime(_)))
            })
    }

    fn claude_add_menu(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        ui.menu_button(RichText::new("+").size(22.0).color(theme::text()), |ui| {
            if ui.button("Add files").clicked() {
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
    }

    fn claude_send_control(&mut self, ui: &mut egui::Ui) -> bool {
        if self.active_chat_running() {
            if icons::button(ui, Icon::Close, "Stop response").clicked() {
                self.stop_response();
            }
        }
        let ready = self.session.active().is_some_and(|chat| {
            !chat.archived && (!chat.draft.trim().is_empty() || !chat.attachments.is_empty())
        });
        ui.add_enabled_ui(ready, |ui| {
            let (rect, response) = ui.allocate_exact_size(vec2(26.0, 24.0), Sense::click());
            let color = if ready { theme::text() } else { theme::muted() };
            let center = rect.center();
            for line in [
                vec![
                    center + vec2(6.0, -6.0),
                    center + vec2(6.0, 1.0),
                    center + vec2(-6.0, 1.0),
                ],
                vec![
                    center + vec2(-2.0, -3.0),
                    center + vec2(-6.0, 1.0),
                    center + vec2(-2.0, 5.0),
                ],
            ] {
                ui.painter()
                    .add(egui::Shape::line(line, Stroke::new(1.5, color)));
            }
            response.on_hover_text("Send message · Enter (Shift+Enter for a new line)")
        })
        .inner
        .clicked()
    }

    fn claude_voice_controls(&self, ui: &mut egui::Ui, microphone: bool) {
        ui.add_enabled_ui(false, |ui| {
            if microphone {
                icons::button(ui, Icon::Mic, "Voice input is not available yet");
            }
            let (rect, response) = ui.allocate_exact_size(vec2(24.0, 26.0), Sense::hover());
            for (index, height) in [4.0, 10.0, 16.0, 8.0, 4.0].into_iter().enumerate() {
                let x = rect.center().x - 8.0 + index as f32 * 4.0;
                ui.painter().line_segment(
                    [
                        pos2(x, rect.center().y - height * 0.5),
                        pos2(x, rect.center().y + height * 0.5),
                    ],
                    Stroke::new(1.2, theme::muted()),
                );
            }
            response.on_disabled_hover_text("Voice mode is not available yet");
        });
    }
}

fn composer_box(
    ui: &mut egui::Ui,
    empty: bool,
    contents: impl FnOnce(&mut egui::Ui),
) -> egui::InnerResponse<()> {
    Frame::new()
        .fill(theme::input_background())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(14)
        .inner_margin(Margin::symmetric(14, if empty { 10 } else { 7 }))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.set_min_width(ui.available_width());
            ui.set_min_height(if empty { 80.0 } else { 22.0 });
            contents(ui);
        })
}

fn claude_heading(ui: &mut egui::Ui, heading: &str) {
    let font = FontId::new(
        if ui.available_width() > 600.0 {
            34.0
        } else {
            28.0
        },
        egui::FontFamily::Name("serif".into()),
    );
    let galley = ui
        .painter()
        .layout_no_wrap(heading.to_owned(), font, theme::text());
    let size = vec2(galley.size().x + 46.0, 44.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let center = pos2(rect.left() + 17.0, rect.center().y);
    for ray in 0..12 {
        let angle = ray as f32 * std::f32::consts::TAU / 12.0;
        let direction = vec2(angle.cos(), angle.sin());
        ui.painter().line_segment(
            [
                center + direction * 5.0,
                center + direction * if ray % 2 == 0 { 16.0 } else { 13.0 },
            ],
            Stroke::new(2.0, theme::blue()),
        );
    }
    ui.painter().galley(
        pos2(rect.left() + 44.0, rect.center().y - galley.size().y * 0.5),
        galley,
        theme::text(),
    );
}
