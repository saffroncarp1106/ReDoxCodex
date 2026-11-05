use eframe::egui::{
    self, Align, Color32, FontId, Frame, Layout, Margin, Rect, RichText, pos2, vec2,
};
use forgedesk::domain::{DEFAULT_MODEL, Mode};

use super::{
    controls::model_label,
    icons::{self, Icon},
    packs, theme,
};
use crate::app::ForgeDesk;

impl ForgeDesk {
    pub(in crate::app) fn gemini_rail(&mut self, context: &egui::Context) {
        egui::SidePanel::left("gemini_rail")
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
                    if icons::button(ui, icon, label).clicked() {
                        match icon {
                            Icon::NewChat => {
                                self.session.new_chat();
                                self.show_archived = false;
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

    pub(in crate::app) fn gemini_conversation(&mut self, context: &egui::Context) {
        let empty = self
            .session
            .active()
            .is_none_or(|chat| chat.messages.is_empty());
        if !empty {
            egui::TopBottomPanel::top("gemini_chat_title")
                .exact_height(44.0)
                .show_separator_line(false)
                .frame(
                    Frame::NONE
                        .fill(theme::background())
                        .inner_margin(Margin::symmetric(24, 9)),
                )
                .show(context, |ui| {
                    let title = self
                        .session
                        .active()
                        .map_or("New chat", |chat| chat.title.as_str());
                    ui.label(RichText::new(title).size(15.0).color(theme::muted()));
                });
            egui::TopBottomPanel::bottom("gemini_composer")
                .show_separator_line(false)
                .frame(
                    Frame::NONE
                        .fill(theme::background())
                        .inner_margin(Margin::symmetric(24, 18)),
                )
                .show(context, |ui| {
                    self.gemini_centered_composer(ui, context);
                });
        }
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(theme::background()).inner_margin(0))
            .show(context, |ui| {
                let rect = ui.max_rect();
                if let Some(texture) = &self.ui_background {
                    ui.painter().image(
                        texture.id(),
                        rect,
                        Rect::from_min_max(egui::Pos2::ZERO, pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                if empty {
                    paint_glow(ui.painter(), rect, &self.active_ui.config);
                    let width = (ui.available_width() - 64.0).clamp(280.0, 780.0);
                    let top = (rect.height() * 0.38 - 55.0).max(28.0);
                    ui.add_space(top);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(&self.active_ui.config.heading)
                                .size(if width > 600.0 { 36.0 } else { 29.0 })
                                .color(theme::text()),
                        );
                        ui.add_space(42.0);
                    });
                    self.gemini_centered_composer(ui, context);
                } else {
                    ui.spacing_mut().scroll = theme::conversation_scroll();
                    egui::ScrollArea::vertical()
                        .id_salt(("gemini_chat", self.session.active_chat))
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

    fn gemini_centered_composer(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        let width = (ui.available_width() - 64.0).clamp(280.0, 780.0);
        let offset = (ui.available_width() - width) * 0.5;
        ui.horizontal_top(|ui| {
            ui.add_space(offset);
            ui.vertical(|ui| {
                ui.set_width(width);
                self.gemini_input(ui, context);
            });
        });
    }

    fn gemini_input(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        let mut send = false;
        if let Some(chat) = self.session.active_mut() {
            let mut remove = None;
            ui.horizontal_wrapped(|ui| {
                for attachment in &chat.attachments {
                    if ui.button(format!("{} ×", attachment.name())).clicked() {
                        remove = Some(attachment.id);
                    }
                }
            });
            if let Some(id) = remove {
                chat.attachments.retain(|attachment| attachment.id != id);
            }
        }
        Frame::new()
            .fill(theme::surface())
            .corner_radius(36)
            .inner_margin(Margin::symmetric(16, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.menu_button(RichText::new("+").size(27.0), |ui| {
                        if ui.button("Add files").clicked() {
                            self.pick_files(context);
                            ui.close();
                        }
                        if ui.button("Open project folder").clicked() {
                            self.pick_workspace(context);
                            ui.close();
                        }
                    });
                    let width = (ui.available_width() - 44.0).max(140.0);
                    if let Some(chat) = self.session.active_mut() {
                        ui.add_enabled_ui(!chat.archived, |ui| {
                            let response = ui.add_sized(
                                [width, 30.0],
                                egui::TextEdit::multiline(&mut chat.draft)
                                    .frame(false)
                                    .font(FontId::proportional(18.0))
                                    .hint_text(&self.active_ui.config.placeholder)
                                    .desired_rows(1)
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
                    if self.running.is_some() {
                        if icons::button(ui, Icon::Close, "Stop response").clicked() {
                            self.stop_response();
                        }
                    } else {
                        let ready = self.session.active().is_some_and(|chat| {
                            !chat.draft.trim().is_empty() || !chat.attachments.is_empty()
                        });
                        if ready {
                            if icons::button(ui, Icon::ArrowUp, "Send message").clicked() {
                                send = true;
                            }
                        } else {
                            ui.add_enabled_ui(false, |ui| {
                                icons::button(ui, Icon::Mic, "Voice input is not available yet");
                            });
                        }
                    }
                });
            });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            let empty = self
                .session
                .active()
                .is_none_or(|chat| chat.messages.is_empty());
            if empty {
                ui.add_space((ui.available_width() * 0.5 - 68.0).max(0.0));
                if let Some(chat) = self.session.active_mut() {
                    ui.add_enabled_ui(self.running.is_none(), |ui| {
                        if ui
                            .selectable_label(chat.mode == Mode::Chat, "Chat")
                            .on_hover_text("Read-only workspace access")
                            .clicked()
                        {
                            chat.set_mode(Mode::Chat);
                        }
                        if ui
                            .selectable_label(chat.mode != Mode::Chat, "Task")
                            .on_hover_text("Workspace coding access")
                            .clicked()
                        {
                            chat.set_mode(Mode::Code);
                        }
                    });
                }
            }
        });
        ui.add_space(5.0);
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("gemini_model")
                .width(145.0)
                .selected_text(model_label(&self.session.model))
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
            self.effort_picker(ui, "gemini_effort");
            if let Some(chat) = self.session.active_mut() {
                let mut mode = chat.mode;
                ui.add_enabled_ui(self.running.is_none(), |ui| {
                    egui::ComboBox::from_id_salt("gemini_access")
                        .width(135.0)
                        .selected_text(mode.label())
                        .show_ui(ui, |ui| {
                            for choice in [Mode::Chat, Mode::Code, Mode::FullAccess] {
                                ui.selectable_value(&mut mode, choice, choice.label());
                            }
                        });
                });
                chat.set_mode(mode);
            }
            ui.label(
                RichText::new("Codex · ChatGPT")
                    .size(11.0)
                    .color(theme::muted()),
            );
        });
        if let Some(error) = &self.attachment_error {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
        if self.picking_files {
            ui.label("Loading files…");
        }
        if self.running.is_some() || (self.models.is_empty() && !self.refreshing) {
            ui.label(RichText::new(&self.status).small().color(theme::muted()));
        }
        if send {
            self.send_prompt(context);
        }
    }
}

fn paint_glow(painter: &egui::Painter, rect: Rect, config: &packs::Config) {
    let color = packs::color(&config.glow).unwrap_or(Color32::BLUE);
    let center = rect.center() + vec2(0.0, 35.0);
    let radius = vec2((rect.width() * 0.48).min(650.0), rect.height() * 0.62);
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(center, color.gamma_multiply(config.glow_strength));
    for i in 0..=64 {
        let angle = i as f32 * std::f32::consts::TAU / 64.0;
        mesh.colored_vertex(
            center + vec2(angle.cos() * radius.x, angle.sin() * radius.y),
            Color32::TRANSPARENT,
        );
        if i > 0 {
            mesh.add_triangle(0, i, i + 1);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}
