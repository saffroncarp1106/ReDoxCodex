use super::icons::{self, Icon};
use super::theme::{self, border, muted, rail, sidebar, text};
use super::win98;
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
                        self.dispatch_ui_action(redox_studio::actions::UiAction::Previous, context);
                    }
                    if icons::button(ui, Icon::Forward, "Next chat").clicked() {
                        self.dispatch_ui_action(redox_studio::actions::UiAction::Next, context);
                    }
                    if icons::button(ui, Icon::Panel, "Toggle sidebar").clicked() {
                        self.sidebar_visible = !self.sidebar_visible;
                    }
                    ui.add_space(14.0);
                    self.main_menus(ui, context);
                    let drag_width = (ui.available_width() - 290.0).max(0.0);
                    let (_, drag) =
                        ui.allocate_exact_size(vec2(drag_width, 30.0), Sense::click_and_drag());
                    super::window_frame::title_drag(context, &drag);

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        self.title_ui_picker(ui);
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

    pub(super) fn main_menus(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        let response = ui.allocate_ui_with_layout(
            vec2(
                184.0,
                if !super::theme::is_classic(ui) {
                    30.0
                } else {
                    20.0
                },
            ),
            Layout::left_to_right(Align::Center),
            |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = if !super::theme::is_classic(ui) {
                        10.0
                    } else {
                        2.0
                    };
                    ui.menu_button(RichText::new("File").color(theme::menu_text()), |ui| {
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
                            self.dispatch_ui_action(
                                redox_studio::actions::UiAction::NewChat,
                                context,
                            );
                            ui.close();
                        }
                        if ui.button("Open project").clicked() {
                            self.dispatch_ui_action(
                                redox_studio::actions::UiAction::OpenProject,
                                context,
                            );
                            ui.close();
                        }
                        if ui.button("Add files").clicked() {
                            self.dispatch_ui_action(
                                redox_studio::actions::UiAction::AddFiles,
                                context,
                            );
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
                    });
                    ui.menu_button(RichText::new("Edit").color(theme::menu_text()), |ui| {
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
                    });
                    ui.menu_button(RichText::new("View").color(theme::menu_text()), |ui| {
                        ui.checkbox(&mut self.sidebar_visible, "Sidebar");
                        ui.checkbox(&mut self.show_archived, "Archived chats");
                        if ui.button("Search chats").clicked() {
                            self.search_open = true;
                            self.sidebar_visible = true;
                            ui.close();
                        }
                    });
                    ui.menu_button(RichText::new("Help").color(theme::menu_text()), |ui| {
                        ui.label("ReDox · Native Rust client");
                        ui.hyperlink_to(
                            "ReDox on GitHub",
                            "https://github.com/saffroncarp1106/ReDoxCodex",
                        );
                    });
                });
            },
        );
        #[cfg(feature = "design-export")]
        super::design_export::record(
            ui.ctx(),
            response.response.rect,
            redox_studio::actions::UiAction::Menus,
        );
        #[cfg(not(feature = "design-export"))]
        let _ = response;
    }

    pub(super) fn title_ui_picker(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.scope(|ui| {
            ui.visuals_mut().override_text_color = Some(theme::menu_text());
            self.ui_picker(ui, "title_ui_picker");
        });
    }

    pub(in crate::app) fn navigate(&mut self, direction: isize) {
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
            .exact_width(52.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                Frame::NONE
                    .fill(rail())
                    .inner_margin(Margin::symmetric(7, 11)),
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
                    self.dispatch_ui_action(redox_studio::actions::UiAction::NewChat, context);
                }
                if rail_button(ui, Icon::Clock, self.show_archived, "Archived chats").clicked() {
                    self.show_archived = true;
                    self.sidebar_visible = true;
                }
                if rail_button(ui, Icon::Books, false, "Open project").clicked() {
                    self.dispatch_ui_action(redox_studio::actions::UiAction::OpenProject, context);
                }
                if ui
                    .add_enabled_ui(self.can_new_chat(), |ui| {
                        rail_button(ui, Icon::NewChat, false, "New coding chat")
                    })
                    .inner
                    .clicked()
                {
                    self.dispatch_ui_action(redox_studio::actions::UiAction::NewChat, context);
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
                top: 11,
                bottom: 11,
            }))
            .show(context, |ui| {
                egui::TopBottomPanel::bottom("sidebar_usage_footer")
                    .resizable(false)
                    .show_separator_line(true)
                    .frame(
                        Frame::NONE
                            .fill(sidebar())
                            .inner_margin(Margin::symmetric(8, 12)),
                    )
                    .show_inside(ui, |ui| self.sidebar_usage(ui, context));
                let classic = theme::is_classic(ui);
                ui.spacing_mut().item_spacing.y = 4.0;
                if self.history_filter.0 != self.show_archived {
                    self.refresh_history(context, false);
                }
                ui.horizontal(|ui| {
                    ui.add_space(1.0);
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
                let new_chat = ui
                    .add_enabled_ui(self.can_new_chat(), |ui| {
                        if classic {
                            win98::button(ui, "New chat…", vec2(ui.available_width(), 24.0))
                        } else {
                            sidebar_row(ui, "New chat", false, 8.0, Some(Icon::NewChat))
                        }
                    })
                    .inner;
                if new_chat.clicked() {
                    self.dispatch_ui_action(redox_studio::actions::UiAction::NewChat, context);
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
                if classic {
                    let history_height = (ui.available_height() * 0.30).clamp(90.0, 240.0);
                    self.history_header(ui, context);
                    sidebar_list(ui, |ui| {
                        super::scroll::ScrollArea::vertical()
                            .id_salt("classic_history_section")
                            .max_height(history_height)
                            .auto_shrink([false, false])
                            .show(ui, |ui| self.history_section(ui, context));
                    });
                    ui.add_space(4.0);
                    egui::TopBottomPanel::bottom("classic_archived_projects")
                        .resizable(false)
                        .min_height(0.0)
                        .max_height(150.0)
                        .show_separator_line(false)
                        .frame(
                            Frame::NONE
                                .fill(sidebar())
                                .inner_margin(Margin::symmetric(0, 4)),
                        )
                        .show_inside(ui, |ui| self.archived_projects_section(ui));
                    section_label(
                        ui,
                        if self.show_archived {
                            "Archived"
                        } else {
                            "Projects"
                        },
                    );
                    sidebar_list(ui, |ui| {
                        super::scroll::ScrollArea::vertical()
                            .id_salt("classic_projects_section")
                            .max_height((ui.available_height() - 6.0).max(0.0))
                            .auto_shrink([false, false])
                            .show(ui, |ui| self.project_section(ui, context));
                    });
                } else {
                    super::scroll::ScrollArea::vertical()
                        .id_salt("sidebar_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            self.history_section(ui, context);
                            sidebar_list(ui, |ui| self.project_section(ui, context));
                        });
                }
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

    fn project_section(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        let query = self.search_query.to_lowercase();
        let archived = self.show_archived;
        let visible = |chat: &&redox::domain::Chat| {
            chat.archived == archived && chat.title.to_lowercase().contains(&query)
        };
        if self.show_archived {
            if !super::theme::is_classic(ui) {
                section_label(ui, "Archived");
            }
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

        if !super::theme::is_classic(ui) {
            section_label(ui, "Projects");
        }
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
            if self.session.archived_projects.contains(&path) {
                continue;
            }
            let name = path
                .file_name()
                .unwrap_or(path.as_os_str())
                .to_string_lossy();
            let project = sidebar_row(ui, &name, false, 8.0, Some(Icon::Folder))
                .on_hover_text(path.display().to_string());
            project.context_menu(|ui| {
                if ui.button("Archive project").clicked() {
                    self.session.archived_projects.push(path.clone());
                    ui.close();
                }
            });
            self.project_drag(ui, &project, &path);
            if project.clicked() {
                self.session.workspace = Some(path.clone());
                if let Some(chat) =
                    self.session
                        .active()
                        .filter(|chat| !chat.archived && chat.workspace.as_ref() == Some(&path))
                        .or_else(|| {
                            self.session.chats.iter().rev().find(|chat| {
                                !chat.archived && chat.workspace.as_ref() == Some(&path)
                            })
                        })
                {
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
                    ui.label(RichText::new("No chats").color(Color32::from_gray(92)));
                });
            }
            self.chat_rows(ui, &chats, 32.0);
            ui.add_space(7.0);
        }

        if sidebar_row(ui, "New project", false, 8.0, Some(Icon::Folder)).clicked() {
            self.dispatch_ui_action(redox_studio::actions::UiAction::OpenProject, context);
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
        if !super::theme::is_classic(ui) {
            self.archived_projects_section(ui);
        }
    }

    fn archived_projects_section(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Archived projects").show(ui, |ui| {
            if let Some(path) = archived_projects_body(ui, &self.session.archived_projects) {
                self.session.archived_projects.retain(|item| item != &path);
            }
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

    fn history_header(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
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
    }

    fn history_section(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        if !super::theme::is_classic(ui) {
            self.history_header(ui, context);
        }
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
                    format!(
                        "[{}] {}",
                        entry
                            .cwd
                            .file_name()
                            .unwrap_or(entry.cwd.as_os_str())
                            .to_string_lossy(),
                        entry.title()
                    ),
                    entry.cwd.display().to_string(),
                )
            })
            .collect();
        let selected = self
            .session
            .active()
            .and_then(|chat| chat.thread_id.clone());
        let rows = |ui: &mut egui::Ui| {
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
                let response = response.on_hover_text(&cwd);
                response.context_menu(|ui| {
                    if ui
                        .add_enabled(
                            !running && !self.show_archived,
                            egui::Button::new("Add to original project"),
                        )
                        .clicked()
                    {
                        self.add_history_to_project(context, &id);
                        ui.close();
                    }
                    ui.label(&cwd);
                });
                if archive.clicked() {
                    self.archive_history_locally(&id);
                    continue;
                }
                if response.clicked() {
                    self.open_history(context, id, false);
                }
            }
        };
        if !super::theme::is_classic(ui) {
            super::scroll::ScrollArea::vertical()
                .id_salt("codex_history_rows")
                .max_height(240.0)
                .auto_shrink([false, true])
                .show(ui, rows);
        } else {
            rows(ui);
        }
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
    let (rect, response) = ui.allocate_exact_size(vec2(38.0, 38.0), Sense::click());
    if selected || response.hovered() {
        ui.painter()
            .rect_filled(rect, 10, Color32::from_gray(if selected { 49 } else { 42 }));
    }
    icons::paint(
        ui.painter(),
        rect.shrink(9.0),
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
    let classic = theme::is_classic(ui);
    ui.horizontal(|ui| {
        let row_height = if classic { 22.0 } else { 30.0 };
        let archive_size = if classic { 20.0 } else { 26.0 };
        let width =
            (ui.available_width() - archive_size - 4.0 - ui.spacing().item_spacing.x).max(20.0);
        let row = ui
            .allocate_ui_with_layout(
                vec2(width, row_height),
                Layout::left_to_right(Align::Center),
                |ui| sidebar_row(ui, title, selected, indent, classic.then_some(Icon::Chat)),
            )
            .inner;
        let archive = ui
            .add_enabled(
                enabled,
                egui::Button::new(if archived { "↶" } else { "×" })
                    .frame(false)
                    .min_size(vec2(archive_size, archive_size)),
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
    let classic = theme::is_classic(ui);
    let row_height = if classic { 22.0 } else { 30.0 };
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), row_height), sense);
    let wii = theme::is_wii(ui.ctx());
    let foreground = if classic && selected {
        Color32::WHITE
    } else {
        text()
    };
    if wii {
        if selected
            || response.hovered()
            || response.has_focus()
            || matches!(icon, Some(Icon::NewChat))
        {
            super::wii::paint_surface(ui, &response, selected);
        }
    } else if selected || response.hovered() {
        ui.painter().rect_filled(
            rect,
            if classic { 0 } else { 10 },
            if classic && selected {
                win98::NAVY
            } else if classic {
                Color32::from_rgb(219, 232, 248)
            } else if selected {
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
            foreground,
        );
        left += 24.0;
    }
    painter.with_clip_rect(rect.shrink2(vec2(8.0, 0.0))).text(
        pos2(left, rect.center().y),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(if classic { 13.0 } else { 14.0 }),
        foreground,
    );
    response.on_hover_text(title)
}

fn archived_projects_body(
    ui: &mut egui::Ui,
    paths: &[std::path::PathBuf],
) -> Option<std::path::PathBuf> {
    let mut restore = None;
    egui::ScrollArea::vertical()
        .id_salt("archived_projects_list")
        .max_height(120.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for path in paths {
                if archived_project_row(ui, path) {
                    restore = Some(path.clone());
                }
            }
        });
    restore
}

fn archived_project_row(ui: &mut egui::Ui, path: &std::path::Path) -> bool {
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), 24.0),
        Layout::right_to_left(Align::Center),
        |ui| {
            let restore = ui.small_button("Restore");
            ui.add(
                egui::Label::new(
                    path.file_name()
                        .unwrap_or(path.as_os_str())
                        .to_string_lossy(),
                )
                .truncate(),
            )
            .on_hover_text(path.display().to_string());
            restore.clicked()
        },
    )
    .inner
}

fn sidebar_list(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui)) {
    if !super::theme::is_classic(ui) {
        contents(ui);
    } else {
        win98::list_box(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            contents(ui);
        });
    }
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
