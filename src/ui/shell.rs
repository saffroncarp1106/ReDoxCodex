use super::icons::{self, Icon};
use super::theme::{self, border, muted, rail, sidebar, surface, text};
use crate::app::ReDox;
use eframe::egui::{
    self, Align, Align2, Color32, FontId, Frame, Layout, Margin, RichText, Sense, Stroke, pos2,
    vec2,
};
use redox::domain::Mode;
use uuid::Uuid;

impl ReDox {
    pub(in crate::app) fn title_bar(&mut self, context: &egui::Context) {
        egui::TopBottomPanel::top("window_chrome")
            .exact_height(42.0)
            .frame(
                Frame::NONE
                    .fill(rail())
                    .inner_margin(Margin::symmetric(4, 5)),
            )
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if icons::button(ui, Icon::Back, "Previous chat").clicked() {
                        self.navigate(-1);
                    }
                    if icons::button(ui, Icon::Forward, "Next chat").clicked() {
                        self.navigate(1);
                    }
                    if icons::button(ui, Icon::Panel, "Toggle sidebar").clicked() {
                        self.sidebar_visible = !self.sidebar_visible;
                    }
                    ui.add_space(14.0);
                    ui.allocate_ui_with_layout(
                        vec2(184.0, 30.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            egui::MenuBar::new().ui(ui, |ui| {
                                ui.spacing_mut().item_spacing.x = 10.0;
                                ui.menu_button(
                                    RichText::new("File").color(theme::menu_text()),
                                    |ui| {
                                        if ui
                                            .add_enabled(
                                                self.can_new_chat(),
                                                egui::Button::new(if cfg!(target_os = "macos") {
                                                    "New chat     ⌘N"
                                                } else {
                                                    "New chat     Ctrl+N"
                                                }),
                                            )
                                            .clicked()
                                        {
                                            self.new_chat();
                                            ui.close();
                                        }
                                        if ui.button("Open project").clicked() {
                                            self.pick_workspace(context);
                                            ui.close();
                                        }
                                        if ui.button("Add files").clicked() {
                                            self.pick_files(context);
                                            ui.close();
                                        }
                                        if ui.button("Settings").clicked() {
                                            self.settings_open = true;
                                            ui.close();
                                        }
                                        ui.separator();
                                        if ui.button("Exit").clicked() {
                                            context.send_viewport_cmd(egui::ViewportCommand::Close);
                                        }
                                    },
                                );
                                ui.menu_button(
                                    RichText::new("Edit").color(theme::menu_text()),
                                    |ui| {
                                        if ui.button("Rename chat").clicked() {
                                            self.rename_open = true;
                                            ui.close();
                                        }
                                        if ui.button("Copy conversation").clicked() {
                                            if let Some(chat) = self.session.active() {
                                                let text = chat
                                                    .messages
                                                    .iter()
                                                    .map(|message| message.content.as_str())
                                                    .collect::<Vec<_>>()
                                                    .join("\n\n");
                                                context.copy_text(text);
                                            }
                                            ui.close();
                                        }
                                    },
                                );
                                ui.menu_button(
                                    RichText::new("View").color(theme::menu_text()),
                                    |ui| {
                                        ui.checkbox(&mut self.sidebar_visible, "Sidebar");
                                        ui.checkbox(&mut self.show_archived, "Archived chats");
                                        if ui.button("Search chats").clicked() {
                                            self.search_open = true;
                                            self.sidebar_visible = true;
                                            ui.close();
                                        }
                                    },
                                );
                                ui.menu_button(
                                    RichText::new("Help").color(theme::menu_text()),
                                    |ui| {
                                        ui.label("ReDox · Native Rust client");
                                        ui.hyperlink_to(
                                            "Codex app-server documentation",
                                            "https://learn.chatgpt.com/docs/app-server",
                                        );
                                    },
                                );
                            });
                        },
                    );
                    ui.scope(|ui| {
                        ui.visuals_mut().override_text_color = Some(theme::menu_text());
                        self.ui_picker(ui, "title_ui_picker");
                    });
                    let drag_width = (ui.available_width() - 108.0).max(0.0);
                    let (_, drag) =
                        ui.allocate_exact_size(vec2(drag_width, 30.0), Sense::click_and_drag());
                    super::window_frame::title_drag(context, &drag);

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::button(ui, Icon::Close, "Close window").clicked() {
                            context.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if icons::button(ui, Icon::Maximize, "Maximize / restore").clicked() {
                            let maximized =
                                context.input(|input| input.viewport().maximized.unwrap_or(false));
                            context.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                        }
                        if icons::button(ui, Icon::Minimize, "Minimize").clicked() {
                            context.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    });
                });
            });
    }

    fn navigate(&mut self, direction: isize) {
        let chats: Vec<_> = self
            .session
            .chats
            .iter()
            .filter(|chat| chat.archived == self.show_archived)
            .map(|chat| chat.id)
            .collect();
        if let Some(index) = chats
            .iter()
            .position(|id| Some(*id) == self.session.active_chat)
        {
            let next = index
                .saturating_add_signed(direction)
                .min(chats.len().saturating_sub(1));
            self.session.active_chat = chats.get(next).copied();
        }
    }

    pub(in crate::app) fn navigation_rail(&mut self, context: &egui::Context) {
        egui::SidePanel::left("navigation_rail")
            .exact_width(48.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                Frame::NONE
                    .fill(rail())
                    .inner_margin(Margin::symmetric(5, 9)),
            )
            .show(context, |ui| {
                ui.spacing_mut().item_spacing.y = 9.0;
                if rail_button(ui, Icon::Home, !self.show_archived, "Home").clicked() {
                    self.show_archived = false;
                    self.sidebar_visible = true;
                }
                if ui
                    .add_enabled_ui(self.can_new_chat(), |ui| {
                        rail_button(ui, Icon::Chat, false, "New chat")
                    })
                    .inner
                    .clicked()
                {
                    self.new_chat();
                }
                if rail_button(ui, Icon::Clock, self.show_archived, "Archived chats").clicked() {
                    self.show_archived = true;
                    self.sidebar_visible = true;
                }
                if rail_button(ui, Icon::Books, false, "Open project").clicked() {
                    self.pick_workspace(context);
                }
                if ui
                    .add_enabled_ui(self.can_new_chat(), |ui| {
                        rail_button(ui, Icon::NewChat, false, "New coding chat")
                    })
                    .inner
                    .clicked()
                {
                    self.new_chat();
                    if let Some(chat) = self.session.active_mut() {
                        chat.mode = Mode::Code;
                    }
                }
                let more = rail_button(ui, Icon::More, false, "More");
                more.context_menu(|ui| {
                    if ui.button("Settings").clicked() {
                        self.settings_open = true;
                        ui.close();
                    }
                    ui.checkbox(&mut self.show_archived, "Archived chats");
                });
                if more.clicked() {
                    self.settings_open = true;
                }
                ui.add_space(1.0);
                ui.separator();
                if rail_button(ui, Icon::Grid, false, "Settings and account").clicked() {
                    self.settings_open = true;
                }

                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    let (rect, response) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
                    ui.painter().circle_filled(
                        rect.center(),
                        12.0,
                        Color32::from_rgb(34, 174, 103),
                    );
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        "C",
                        FontId::proportional(12.0),
                        Color32::WHITE,
                    );
                    if response.on_hover_text(&self.account).clicked() {
                        self.settings_page = super::SettingsPage::Account;
                        self.settings_open = true;
                    }
                    ui.add_space(12.0);
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

    pub(in crate::app) fn sidebar(&mut self, context: &egui::Context) {
        egui::SidePanel::left("project_sidebar")
            .default_width(289.0)
            .width_range(230.0..=360.0)
            .show_separator_line(false)
            .frame(Frame::NONE.fill(sidebar()).inner_margin(Margin {
                left: 11,
                right: 11,
                top: 14,
                bottom: 11,
            }))
            .show(context, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                if self.history_filter.0 != self.show_archived {
                    self.refresh_history(context, false);
                }
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    let mode = &mut self.ui_preferences.codex_mode;
                    egui::ComboBox::from_id_salt("chat_presentation")
                        .width(96.0)
                        .selected_text(
                            RichText::new(if *mode { "Codex" } else { "ChatGPT" })
                                .font(theme::semibold(17.0)),
                        )
                        .show_ui(ui, |ui| {
                            ui.selectable_value(mode, false, "ChatGPT");
                            ui.selectable_value(mode, true, "Codex");
                        })
                        .response
                        .on_hover_text("Choose how much activity to show in chat");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::button(ui, Icon::Search, "Search chats").clicked() {
                            self.search_open = !self.search_open;
                        }
                        if icons::button(ui, Icon::Bell, "Account status").clicked() {
                            self.settings_page = super::SettingsPage::Account;
                            self.settings_open = true;
                        }
                    });
                });
                ui.add_space(8.0);
                if ui
                    .add_enabled_ui(self.can_new_chat(), |ui| {
                        sidebar_row(ui, "New chat", false, 8.0, Some(Icon::NewChat))
                    })
                    .inner
                    .clicked()
                {
                    self.new_chat();
                }

                if self.search_open {
                    ui.add_space(4.0);
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("Search chats")
                            .desired_width(f32::INFINITY),
                    );
                    if response.lost_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter))
                    {
                        self.refresh_history(context, false);
                    }
                }

                ui.add_space(5.0);
                egui::ScrollArea::vertical()
                    .id_salt("sidebar_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.history_section(ui, context);
                        let query = self.search_query.to_lowercase();
                        let archived = self.show_archived;
                        let visible = |chat: &&redox::domain::Chat| {
                            chat.archived == archived && chat.title.to_lowercase().contains(&query)
                        };
                        if self.show_archived {
                            section_label(ui, "Archived");
                            let chats: Vec<_> = self
                                .session
                                .chats
                                .iter()
                                .rev()
                                .filter(visible)
                                .map(|chat| (chat.id, chat.title.clone()))
                                .collect();
                            if chats.is_empty() {
                                ui.label(RichText::new("No archived chats").color(muted()));
                            }
                            self.chat_rows(ui, &chats, 20.0);
                            return;
                        }

                        section_label(ui, "Projects");
                        let mut projects = Vec::new();
                        if let Some(path) = &self.session.workspace {
                            projects.push(path.clone());
                        }
                        for chat in &self.session.chats {
                            if let Some(path) = &chat.workspace
                                && !projects.contains(path)
                            {
                                projects.push(path.clone());
                            }
                        }

                        for path in projects {
                            let name = path
                                .file_name()
                                .unwrap_or(path.as_os_str())
                                .to_string_lossy();
                            let project = sidebar_row(ui, &name, false, 8.0, Some(Icon::Folder))
                                .on_hover_text(path.display().to_string());
                            self.project_drag(ui, &project, &path);
                            if project.clicked() {
                                self.session.workspace = Some(path.clone());
                                if let Some(chat) = self.session.chats.iter().rev().find(|chat| {
                                    !chat.archived && chat.workspace.as_ref() == Some(&path)
                                }) {
                                    self.session.active_chat = Some(chat.id);
                                } else {
                                    self.new_project_chat(path.clone());
                                }
                            }
                            let chats: Vec<_> = self
                                .session
                                .chats
                                .iter()
                                .rev()
                                .filter(visible)
                                .filter(|chat| chat.workspace.as_ref() == Some(&path))
                                .map(|chat| (chat.id, chat.title.clone()))
                                .collect();
                            if chats.is_empty() {
                                ui.horizontal(|ui| {
                                    ui.add_space(32.0);
                                    ui.label(
                                        RichText::new("No chats").color(Color32::from_gray(92)),
                                    );
                                });
                            }
                            self.chat_rows(ui, &chats, 32.0);
                            ui.add_space(7.0);
                        }

                        if sidebar_row(ui, "New project", false, 8.0, Some(Icon::Folder)).clicked()
                        {
                            self.pick_workspace(context);
                        }
                        ui.add_space(25.0);
                        section_label(ui, "Recents");
                        let chats: Vec<_> = self
                            .session
                            .chats
                            .iter()
                            .rev()
                            .filter(visible)
                            .map(|chat| (chat.id, chat.title.clone()))
                            .collect();
                        self.chat_rows(ui, &chats, 8.0);
                    });
                let right = ui.max_rect().right() + 8.0;
                ui.painter().line_segment(
                    [
                        pos2(right, ui.max_rect().top() - 8.0),
                        pos2(right, ui.max_rect().bottom() + 8.0),
                    ],
                    Stroke::new(1.0_f32, border()),
                );
            });
    }

    fn chat_rows(&mut self, ui: &mut egui::Ui, chats: &[(Uuid, String)], indent: f32) {
        for (id, title) in chats {
            let running = self
                .session
                .chats
                .iter()
                .find(|chat| chat.id == *id)
                .is_some_and(|chat| {
                    chat.messages
                        .iter()
                        .any(|message| message.status == redox::domain::MessageStatus::Streaming)
                });
            let (response, archive) = chat_row(
                ui,
                title,
                self.session.active_chat == Some(*id),
                indent,
                self.show_archived,
                !running,
            );
            if archive.clicked() {
                self.archive_local_chat(*id, !self.show_archived);
                continue;
            }
            if response.clicked() {
                self.session.active_chat = Some(*id);
                self.focus_prompt = self.session.active().is_some_and(|chat| chat.is_blank());
                if let Some(thread) = self
                    .session
                    .active()
                    .and_then(|chat| chat.thread_id.clone())
                {
                    self.open_history(ui.ctx(), thread, false);
                }
            }
            response.context_menu(|ui| {
                if ui.button("Rename").clicked() {
                    self.session.active_chat = Some(*id);
                    self.rename_open = true;
                    ui.close();
                }
                let label = if self.show_archived {
                    "Restore locally"
                } else {
                    "Archive locally"
                };
                if ui.button(label).clicked() {
                    self.archive_local_chat(*id, !self.show_archived);
                    ui.close();
                }
            });
        }
    }

    fn history_section(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Codex history")
                    .font(theme::semibold(14.0))
                    .color(muted()),
            );
            if ui
                .add_enabled(!self.history_loading, egui::Button::new("Refresh"))
                .clicked()
            {
                self.refresh_history(context, false);
            }
        });
        if self.search_open
            && self.history_filter.1 != self.search_query
            && ui.small_button("Search Codex history").clicked()
        {
            self.refresh_history(context, false);
        }
        if let Some(error) = &self.history_error {
            ui.label(
                RichText::new(error)
                    .small()
                    .color(egui::Color32::from_rgb(235, 145, 125)),
            );
        }
        let entries: Vec<_> = self
            .history
            .iter()
            .filter(|entry| {
                self.session
                    .local_archive_overrides
                    .get(&entry.id)
                    .is_none_or(|archived| *archived == self.show_archived)
            })
            .map(|entry| {
                (
                    entry.id.clone(),
                    entry.title(),
                    entry.cwd.display().to_string(),
                )
            })
            .collect();
        let selected = self
            .session
            .active()
            .and_then(|chat| chat.thread_id.clone());
        egui::ScrollArea::vertical()
            .id_salt("codex_history_rows")
            .max_height(240.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for (id, title, cwd) in entries {
                    let title = if title.is_empty() {
                        "Codex conversation"
                    } else {
                        &title
                    };
                    let running = self
                        .session
                        .chats
                        .iter()
                        .filter(|chat| chat.thread_id.as_deref() == Some(id.as_str()))
                        .any(|chat| {
                            chat.messages.iter().any(|message| {
                                message.status == redox::domain::MessageStatus::Streaming
                            })
                        });
                    let (response, archive) = chat_row(
                        ui,
                        title,
                        selected.as_ref() == Some(&id),
                        8.0,
                        false,
                        !running && !self.show_archived,
                    );
                    let response = response.on_hover_text(cwd);
                    if archive.clicked() {
                        self.archive_history_locally(&id);
                        continue;
                    }
                    if response.clicked() {
                        self.open_history(context, id, false);
                    }
                }
            });
        if self.history_loading || self.history_reading.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading Codex history...");
            });
        } else if self.history.is_empty() && self.history_error.is_none() {
            ui.label(
                RichText::new("No saved Codex chats found")
                    .small()
                    .color(muted()),
            );
        }
        if self.history_cursor.is_some()
            && !self.history_loading
            && ui.small_button("Load more chats").clicked()
        {
            self.refresh_history(context, true);
        }
        if let Some(thread) = selected
            && self
                .conversation_cursors
                .get(&thread)
                .is_some_and(Option::is_some)
            && ui
                .add_enabled(
                    self.history_reading.is_none() && self.running.is_none(),
                    egui::Button::new("Load older messages"),
                )
                .clicked()
        {
            self.open_history(context, thread, true);
        }
        ui.add_space(12.0);
    }
}

fn rail_button(ui: &mut egui::Ui, icon: Icon, selected: bool, tooltip: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(36.0, 35.0), Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(rect, 10, surface());
    }
    icons::paint(
        ui.painter(),
        rect.shrink2(vec2(10.0, 9.0)),
        icon,
        if selected { Color32::WHITE } else { muted() },
    );
    response.on_hover_text(tooltip)
}

fn chat_row(
    ui: &mut egui::Ui,
    title: &str,
    selected: bool,
    indent: f32,
    archived: bool,
    enabled: bool,
) -> (egui::Response, egui::Response) {
    ui.horizontal(|ui| {
        let width = (ui.available_width() - 30.0 - ui.spacing().item_spacing.x).max(20.0);
        let row = ui
            .allocate_ui_with_layout(
                vec2(width, 30.0),
                Layout::left_to_right(Align::Center),
                |ui| sidebar_row(ui, title, selected, indent, None),
            )
            .inner;
        let archive = ui
            .add_enabled(
                enabled,
                egui::Button::new(if archived { "↶" } else { "×" })
                    .frame(false)
                    .min_size(vec2(26.0, 26.0)),
            )
            .on_hover_text(if archived {
                "Restore locally"
            } else {
                "Archive locally"
            });
        (row, archive)
    })
    .inner
}

fn sidebar_row(
    ui: &mut egui::Ui,
    title: &str,
    selected: bool,
    indent: f32,
    icon: Option<Icon>,
) -> egui::Response {
    let mut painter = ui.painter().clone();
    if matches!(icon, Some(Icon::NewChat)) {
        painter.set_opacity(1.0);
    }
    let sense = if matches!(icon, Some(Icon::Folder)) {
        Sense::click_and_drag()
    } else {
        Sense::click()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), sense);
    if selected || response.hovered() {
        ui.painter().rect_filled(
            rect,
            10,
            if selected {
                Color32::from_gray(46)
            } else {
                Color32::from_gray(38)
            },
        );
    }
    let mut left = rect.left() + indent;
    if let Some(icon) = icon {
        icons::paint(
            &painter,
            egui::Rect::from_center_size(pos2(left + 8.0, rect.center().y), vec2(16.0, 16.0)),
            icon,
            text(),
        );
        left += 24.0;
    }
    painter.with_clip_rect(rect.shrink2(vec2(8.0, 0.0))).text(
        pos2(left, rect.center().y),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(14.0),
        text(),
    );
    response.on_hover_text(title)
}

fn section_label(ui: &mut egui::Ui, title: &str) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(
            RichText::new(title)
                .font(theme::semibold(14.0))
                .color(muted()),
        );
    });
    ui.add_space(6.0);
}

#[cfg(test)]
#[path = "../../tests/ui/chat_rows.rs"]
mod chat_row_tests;
