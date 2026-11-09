use eframe::egui::{self, Align, Align2, Color32, Frame, Layout, Margin, RichText, Stroke, vec2};

use super::controls::model_label;
use super::icons::{self, Icon};
use super::theme::{muted, surface};
use super::{Preferences, SettingsPage, theme, win98};
use crate::app::ReDox;
use redox::codex;
use redox::domain::{DEFAULT_MODEL, Mode};

const SETTINGS_PAGES: [(SettingsPage, &str); 8] = [
    (SettingsPage::Account, "Account"),
    (SettingsPage::Models, "Models & reasoning"),
    (SettingsPage::Workspace, "Workspace & access"),
    (SettingsPage::Appearance, "Appearance"),
    (SettingsPage::Cursors, "Cursor animation"),
    (SettingsPage::Advanced, "Advanced"),
    (SettingsPage::Updates, "Updates"),
    (SettingsPage::About, "About"),
];

const SETTINGS_ICONS: [Icon; SETTINGS_PAGES.len()] = [
    Icon::Home,
    Icon::Bolt,
    Icon::Shield,
    Icon::Grid,
    Icon::Reticle,
    Icon::Settings,
    Icon::Bolt,
    Icon::Chat,
];

impl ReDox {
    pub(in crate::app) fn settings(&mut self, context: &egui::Context) {
        if self.active_ui.config.layout == win98::ID {
            self.win98_settings(context);
            return;
        }
        let mut open = self.settings_open;
        let animation_was_active = self.startup.is_active();
        if self.ui_preferences.floating_windows {
            if !open || animation_was_active {
                return;
            }
            super::window_frame::native_popup(
                context,
                "native_settings",
                "Settings",
                vec2(800.0, 590.0),
                &mut open,
                |ui| {
                    let child = ui.ctx().clone();
                    self.settings_body(ui, &child);
                },
            );
            self.settings_open = open && self.settings_open && !self.startup.is_active();
            return;
        }
        let available = context.content_rect().size();
        let size = vec2(
            (available.x - 40.0).clamp(420.0, 780.0),
            (available.y - 90.0).clamp(280.0, 550.0),
        );
        egui::Window::new("Settings")
            .id(egui::Id::new("settings_panel_v3"))
            .default_pos(context.content_rect().center() - size * 0.5)
            .open(&mut open)
            .fixed_size(size)
            .resizable(false)
            .collapsible(false)
            .show(context, |ui| {
                self.settings_body(ui, context);
            });
        self.settings_open = open && (animation_was_active || !self.startup.is_active());
    }

    fn settings_body(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        ui.add_space(6.0);
        let pages = SETTINGS_PAGES;
        if ui.available_width() < 560.0 {
            let title = pages
                .iter()
                .find(|(page, _)| *page == self.settings_page)
                .map_or("Account", |(_, title)| *title);
            egui::ComboBox::from_id_salt("settings_section")
                .width(180.0)
                .selected_text(title)
                .show_ui(ui, |ui| {
                    for (page, label) in pages {
                        ui.selectable_value(&mut self.settings_page, page, label);
                    }
                });
            ui.separator();
            self.settings_content(ui, context);
        } else {
            ui.horizontal_top(|ui| {
                Frame::NONE
                    .fill(theme::rail())
                    .stroke(Stroke::new(1.0, theme::border()))
                    .corner_radius(10)
                    .inner_margin(8)
                    .show(ui, |ui| {
                        ui.allocate_ui_with_layout(
                            vec2(166.0, 0.0),
                            Layout::top_down(Align::Min),
                            |ui| {
                                ui.spacing_mut().item_spacing.y = 3.0;
                                for ((page, label), icon) in pages.into_iter().zip(SETTINGS_ICONS) {
                                    if settings_tab(ui, label, icon, self.settings_page == page)
                                        .clicked()
                                    {
                                        self.settings_page = page;
                                    }
                                }
                            },
                        );
                    });
                Frame::NONE
                    .inner_margin(Margin::symmetric(10, 4))
                    .show(ui, |ui| {
                        self.settings_content(ui, context);
                    });
            });
        }
    }

    fn win98_settings(&mut self, context: &egui::Context) {
        if !self.settings_open || self.startup.is_active() {
            return;
        }
        let available = context.content_rect().size();
        let size = vec2(
            (available.x - 28.0).clamp(320.0, 566.0),
            (available.y - 42.0).clamp(270.0, 520.0),
        );
        if self.ui_preferences.floating_windows {
            context.show_viewport_immediate(
                egui::ViewportId::from_hash_of("win98_settings"),
                Self::app_viewport()
                    .with_title("ReDox Properties")
                    .with_decorations(false)
                    .with_resizable(false)
                    .with_inner_size(size),
                |child, class| {
                    if class == egui::ViewportClass::Embedded {
                        self.win98_settings_window(child, size);
                    } else if child.input(|input| input.viewport().close_requested())
                        || (child.input(|input| input.key_pressed(egui::Key::Escape))
                            && !egui::Popup::is_any_open(child))
                    {
                        self.settings_open = false;
                    } else {
                        egui::CentralPanel::default()
                            .frame(Frame::NONE)
                            .show(child, |ui| {
                                self.win98_property_sheet(ui, true);
                            });
                    }
                    if !self.settings_open || child.input(|input| !input.events.is_empty()) {
                        child.request_repaint_of(egui::ViewportId::ROOT);
                    }
                },
            );
        } else {
            self.win98_settings_window(context, size);
        }
    }

    fn win98_settings_window(&mut self, context: &egui::Context, size: egui::Vec2) {
        egui::Window::new("ReDox Properties")
            .id(egui::Id::new("win98_properties_sheet"))
            .default_pos(context.content_rect().center() - size * 0.5)
            .fixed_size(size)
            .title_bar(false)
            .frame(Frame::NONE)
            .show(context, |ui| self.win98_property_sheet(ui, false));
    }

    fn win98_property_sheet(&mut self, ui: &mut egui::Ui, floating: bool) {
        let context = ui.ctx().clone();
        let shell = Frame::NONE
            .fill(theme::sidebar())
            .inner_margin(4)
            .show(ui, |ui| {
                let (caption, drag) = ui.allocate_exact_size(
                    vec2(ui.available_width(), 24.0),
                    if floating {
                        egui::Sense::drag()
                    } else {
                        egui::Sense::hover()
                    },
                );
                win98::caption_background(ui.painter(), caption);
                ui.painter().text(
                    caption.left_center() + vec2(6.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    "ReDox Properties",
                    theme::semibold(14.0),
                    Color32::WHITE,
                );
                if floating && drag.drag_started() {
                    context.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                let close_rect = egui::Rect::from_center_size(
                    caption.right_center() - vec2(12.0, 0.0),
                    vec2(19.0, 19.0),
                );
                let close = ui
                    .put(
                        close_rect,
                        egui::Button::new("×").min_size(vec2(19.0, 19.0)),
                    )
                    .on_hover_text("Close Settings");
                win98::bevel(ui.painter(), close.rect, close.is_pointer_button_down_on());
                if close.clicked() {
                    self.settings_open = false;
                }
                Frame::NONE.inner_margin(8).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let mut selected_tab = None;
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(1.0, 2.0);
                        for ((page, tooltip), label) in SETTINGS_PAGES.into_iter().zip([
                            "Account",
                            "Models",
                            "Workspace",
                            "Appearance",
                            "Cursors",
                            "Advanced",
                            "Updates",
                            "About",
                        ]) {
                            let selected = self.settings_page == page;
                            let response = ui
                                .add(
                                    egui::Button::new(label)
                                        .fill(if selected {
                                            theme::sidebar()
                                        } else {
                                            Color32::from_gray(176)
                                        })
                                        .min_size(vec2(0.0, 24.0)),
                                )
                                .on_hover_text(tooltip);
                            win98::bevel(ui.painter(), response.rect, false);
                            if selected {
                                selected_tab = Some(response.rect);
                            }
                            if response.clicked() {
                                self.settings_page = page;
                            }
                        }
                    });
                    let pane = Frame::NONE.inner_margin(12).show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 5.0;
                        self.settings_content(ui, &context);
                    });
                    win98::bevel(ui.painter(), pane.response.rect, false);
                    if let Some(tab) = selected_tab
                        .filter(|tab| (pane.response.rect.top() - tab.bottom()).abs() <= 1.0)
                    {
                        let join = egui::Rect::from_min_max(
                            tab.left_bottom() + vec2(1.0, -1.0),
                            tab.right_bottom() + vec2(-1.0, 1.5),
                        );
                        ui.painter().rect_filled(join, 0, theme::sidebar());
                    }
                    ui.add_space(8.0);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if win98::button(ui, "OK", vec2(78.0, 25.0)).clicked() {
                            self.settings_open = false;
                        }
                        ui.label(
                            RichText::new("Changes apply as you make them.")
                                .small()
                                .color(muted()),
                        );
                    });
                });
            });
        win98::bevel(ui.painter(), shell.response.rect, false);
    }

    fn settings_content(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        let classic = self.active_ui.config.layout == win98::ID;
        theme::panel_scroll(ui);
        super::scroll::ScrollArea::vertical()
            .id_salt(("settings_content", self.settings_page as u8))
            .max_height(if classic {
                (ui.available_height() - 44.0).max(0.0)
            } else {
                (ui.available_height() - 6.0).max(0.0)
            })
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_min_width(ui.available_width());
                    ui.add_space(4.0);
                    match self.settings_page {
                        SettingsPage::Account => self.account_settings(ui, context),
                        SettingsPage::Models => self.model_settings(ui),
                        SettingsPage::Workspace => self.workspace_settings(ui, context),
                        SettingsPage::Appearance => self.appearance_settings(ui, context),
                        SettingsPage::Cursors => {
                            heading(
                                ui,
                                "Cursor animation",
                                "Choose the names and colors in your startup animation.",
                            );
                            if self.startup.settings(ui) {
                                self.settings_open = false;
                            }
                        }
                        SettingsPage::Advanced => self.advanced_settings(ui, context),
                        SettingsPage::Updates => self.updates_settings(ui),
                        SettingsPage::About => self.about_settings(ui),
                    }
                });
            });
    }

    fn account_settings(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        heading(
            ui,
            "Account",
            "Connect your ChatGPT account and manage sign-in.",
        );
        theme::card(ui, |ui| {
            let (label, color) = if self.refreshing {
                ("Connecting…", theme::blue())
            } else if self.signed_in {
                ("Connected", Color32::from_rgb(43, 155, 101))
            } else {
                ("Not connected", muted())
            };
            theme::status_line(ui, RichText::new(label).strong(), color);
            ui.label(RichText::new(&self.account).color(muted()));
        });
        ui.add_space(4.0);
        if ui
            .add_enabled(
                !self.refreshing && !self.logging_out,
                egui::Button::new("Refresh connection"),
            )
            .clicked()
        {
            self.refresh_models(context);
        }
        if self.refreshing {
            ui.spinner();
        }
        ui.add_space(16.0);
        let signing_in = self
            .login_task
            .as_ref()
            .is_some_and(|task| !task.is_finished());
        let label = if self.signed_in {
            "Sign in to another account"
        } else {
            "Sign in with ChatGPT"
        };
        if ui
            .add_enabled(!signing_in && !self.logging_out, egui::Button::new(label))
            .clicked()
        {
            self.start_login(context);
        }
        if self.signed_in {
            ui.add_space(8.0);
            let label = if self.logging_out {
                "Logging out..."
            } else {
                "Log out"
            };
            if ui
                .add_enabled(self.can_logout(), egui::Button::new(label))
                .on_disabled_hover_text(
                    "Wait for the current response, sign-in, or account refresh to finish.",
                )
                .clicked()
            {
                self.logout(context);
            }
            ui.label(
                RichText::new(
                    "Logs out of the Codex account shared with your CLI. Saved chats are kept.",
                )
                .small()
                .color(muted()),
            );
        }
        if !self.login_text.is_empty() {
            ui.label(&self.login_text);
        }
        if let Some(challenge) = &self.login_challenge {
            theme::card(ui, |ui| {
                ui.label(RichText::new("Verification code").color(muted()));
                ui.add(
                    egui::Label::new(RichText::new(&challenge.user_code).monospace().size(25.0))
                        .selectable(true),
                );
                ui.horizontal(|ui| {
                    if ui.button("Copy code").clicked() {
                        context.copy_text(challenge.user_code.clone());
                    }
                    ui.hyperlink_to("Open sign-in page", &challenge.verification_url);
                });
            });
        }
        if signing_in && ui.button("Cancel sign-in").clicked() {
            if let Some(task) = self.login_task.take() {
                task.abort();
            }
            self.login_challenge = None;
            self.login_receiver = None;
            self.login_text = "Sign-in cancelled.".into();
        }
        ui.add_space(20.0);
        ui.label(RichText::new("Usage limits").font(theme::semibold(14.0)));
        ui.label(RichText::new("Use the reticle button above your profile at the bottom left to see remaining quota and reset times.").color(muted()));
        ui.add_space(12.0);
        ui.label(RichText::new(&self.status).small().color(muted()));
    }

    fn model_settings(&mut self, ui: &mut egui::Ui) {
        heading(
            ui,
            "Models & reasoning",
            "Set the model and reasoning effort for your next message.",
        );
        settings_group(ui, "Model", |ui| {
            egui::ComboBox::from_id_salt("settings_model")
                .width(ui.available_width().min(300.0))
                .selected_text(model_label(&self.session.model))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.session.model,
                        DEFAULT_MODEL.into(),
                        DEFAULT_MODEL,
                    );
                    for model in &self.models {
                        let selected = self.session.model == *model;
                        if ui.selectable_label(selected, model_label(model)).clicked() {
                            self.session.model.clone_from(model);
                        }
                    }
                });
        });
        settings_group(ui, "Reasoning effort", |ui| {
            self.effort_picker(ui, "settings_effort");
            if let Some(model) = self.selected_model_info() {
                let selected = self.selected_effort();
                if let Some(option) = model
                    .supported_reasoning_efforts
                    .iter()
                    .find(|option| Some(&option.reasoning_effort) == selected.as_ref())
                {
                    ui.label(RichText::new(&option.description).color(muted()));
                }
            }
            ui.add_space(12.0);
            ui.label(RichText::new("Your effort choice is remembered separately for each model. Only supported levels are offered.").color(muted()));
        });
        ui.add_space(18.0);
        ui.label("You can also change the model and effort directly in the composer.");
    }

    fn workspace_settings(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        heading(
            ui,
            "Workspace & access",
            "Choose a project and what the current chat can do.",
        );
        settings_group(ui, "Project folder", |ui| {
            if let Some(path) = self
                .session
                .active()
                .and_then(|chat| chat.workspace.as_ref())
            {
                ui.add(
                    egui::Label::new(RichText::new(path.display().to_string()).monospace())
                        .wrap()
                        .selectable(true),
                );
            } else {
                ui.label(RichText::new("No project folder for this chat").color(muted()));
            }
            if ui
                .add_enabled(
                    !self.picking_workspace,
                    egui::Button::new("Choose project folder"),
                )
                .clicked()
            {
                self.pick_workspace(context);
            }
            ui.label(
                RichText::new("Opening a project creates or selects a chat for that folder. If this chat's folder is missing, it reconnects this chat and keeps your draft.")
                    .small()
                    .color(muted()),
            );
            let selected = self.session.workspace.clone();
            if self
                .session
                .active()
                .is_some_and(|chat| chat.workspace != selected)
                && selected.as_ref().is_some_and(|path| path.is_dir())
                && ui
                    .add_enabled(
                        self.running.is_none(),
                        egui::Button::new("Use selected project for this chat"),
                    )
                    .clicked()
                && let Some(chat) = self.session.active_mut()
            {
                chat.set_workspace(selected);
            }
            if let Some(path) = self
                .session
                .active()
                .and_then(|chat| chat.workspace.as_ref())
                && !path.is_dir()
            {
                ui.colored_label(Color32::LIGHT_RED, "This chat's project folder no longer exists. Select a valid project and use it for this chat.");
            }
        });
        ui.add_space(20.0);
        settings_group(ui, "ReDox Remote control", |ui| {
            ui.add_enabled_ui(redox::desktop::SUPPORTED && self.running.is_none(), |ui| {
                ui.checkbox(
                    &mut self.session.desktop.enabled,
                    "Allow the selected agent to see and control this desktop",
                );
                ui.add_enabled_ui(self.session.desktop.enabled, |ui| {
                    ui.checkbox(
                        &mut self.session.desktop.confirm_actions,
                        "Ask before mouse and keyboard input outside Full access",
                    );
                });
            });
            ui.label(RichText::new(if redox::desktop::SUPPORTED {
                "ReDox lets Codex and Ollama capture screens, focus windows, and use the mouse and keyboard. Full access skips the remote input prompt when Remote control is enabled. Ollama requires vision and tool calling. Screenshots go to the selected provider. Use Stop or hold Escape to stop remote input. Changes apply on the next response."
            } else {
                "ReDox Remote control currently supports Windows."
            }).small().color(muted()));
        });
        ui.add_space(20.0);
        settings_group(ui, "Current chat permissions", |ui| {
            if let Some(chat) = self.session.active_mut() {
                let mut mode = chat.mode;
                ui.add_enabled_ui(self.running.is_none(), |ui| {
                    for (option, detail) in [
                        (Mode::Chat, "Read files and run read-only commands."),
                        (Mode::Code, "Read and edit project files; review commands when Codex requests approval."),
                        (Mode::FullAccess, "Unrestricted filesystem and commands, without approval prompts."),
                    ] {
                        ui.radio_value(&mut mode, option, option.label());
                        ui.label(RichText::new(detail).small().color(muted()));
                        ui.add_space(5.0);
                    }
                });
                chat.set_mode(mode);
            }
        });
        ui.add_space(12.0);
        ui.label(RichText::new("Attachments").font(theme::semibold(14.0)));
        ui.label(RichText::new("Use + → Add files in the composer. Up to 8 files per message; 128 KiB per text file and 20 MiB per image.").color(muted()));
    }

    fn appearance_settings(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        heading(
            ui,
            "Appearance",
            "Adjust the interface without changing your chats.",
        );
        settings_group(ui, "Interface", |ui| {
            ui.horizontal_wrapped(|ui| {
                self.ui_picker(ui, "settings_ui_picker");
                self.skin_settings(ui, context);
            });
            if let Some(error) = &self.ui_error {
                ui.colored_label(Color32::LIGHT_RED, error);
            }
            for error in &self.ui_catalog.errors {
                ui.colored_label(Color32::LIGHT_RED, error);
            }
        });
        settings_group(ui, "Project folder border glow", |ui| {
            self.ui_preferences.project_glow.settings(ui)
        });
        ui.add_space(12.0);
        settings_group(ui, "Text colors", |ui| {
            let base = self.active_ui.config.colors().unwrap_or_default();
            let mut colors = self
                .ui_preferences
                .text_colors
                .get(&self.active_ui.id)
                .cloned()
                .unwrap_or_default();
            let mut changed = false;
            for (label, choice, fallback) in [
                ("Top row text", &mut colors.menu, base.menu_text),
                ("General text", &mut colors.general, base.text),
                (
                    "Text box background",
                    &mut colors.input_background,
                    base.input_background,
                ),
            ] {
                ui.horizontal(|ui| {
                    let mut rgb = choice.unwrap_or([fallback.r(), fallback.g(), fallback.b()]);
                    if ui.color_edit_button_srgb(&mut rgb).changed() {
                        *choice = Some(rgb);
                        changed = true;
                    }
                    ui.label(label);
                    ui.label(
                        RichText::new(format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]))
                            .small()
                            .color(muted()),
                    );
                });
            }
            ui.label(RichText::new("Top row controls File, Edit, View, Help, and the UI selector. General text controls the main interface and chat text. Changes apply live and are saved separately for each layout.").small().color(muted()));
            if ui.button("Restore text colors").clicked() {
                colors = theme::TextColors::default();
                changed = true;
            }
            if changed {
                self.ui_preferences
                    .text_colors
                    .insert(self.active_ui.id.clone(), colors);
                context.request_repaint_of(egui::ViewportId::ROOT);
            }
        });
        ui.add_space(12.0);
        settings_group(ui, "Window and scale", |ui| {
            let mut percent = self.ui_preferences.zoom * 100.0;
            let changed = ui
                .scope(|ui| {
                    ui.spacing_mut().slider_width = 190.0;
                    if ui.visuals().dark_mode {
                        let visuals = &mut ui.style_mut().visuals;
                        visuals.widgets.inactive.bg_fill = theme::blue();
                        visuals.widgets.inactive.weak_bg_fill = Color32::from_gray(75);
                        visuals.widgets.inactive.bg_stroke =
                            egui::Stroke::new(1.0_f32, theme::blue());
                        visuals.widgets.hovered.bg_fill = theme::blue();
                        visuals.widgets.active.bg_fill = theme::blue();
                    }
                    theme::slider(
                        ui,
                        egui::Slider::new(&mut percent, 80.0..=150.0)
                            .suffix("%")
                            .step_by(5.0),
                    )
                    .changed()
                })
                .inner;
            if changed {
                self.ui_preferences.zoom = percent / 100.0;
                context.set_zoom_factor(self.ui_preferences.zoom);
            }
            ui.add_space(16.0);
            ui.checkbox(&mut self.sidebar_visible, "Show project and chat sidebar");
            if ui
                .checkbox(
                    &mut self.ui_preferences.floating_windows,
                    "Allow popup windows outside the app",
                )
                .changed()
            {
                context.request_repaint_of(egui::ViewportId::ROOT);
            }
            ui.label(RichText::new("Settings, usage, rename, and approval dialogs open as separate desktop windows. Turn this off to keep them inside the app.").small().color(muted()));
            ui.add_space(16.0);
            if ui.button("Restore default appearance").clicked() {
                self.ui_preferences.text_colors.remove(&self.active_ui.id);
                context.request_repaint_of(egui::ViewportId::ROOT);
                self.ui_preferences.zoom = Preferences::default().zoom;
                self.sidebar_visible = true;
                context.set_zoom_factor(1.0);
            }
        });
        ui.add_space(25.0);
        settings_group(ui, "Custom interfaces", |ui| {
            ui.label(RichText::new("Drop JSON configs or folders containing ui.json into src/ui/layouts/. Available packs appear in the UI menu automatically. Config edits apply after selecting the UI and restarting.").color(muted()));
            if ui.button("Copy UI packs folder").clicked() {
                context.copy_text(self.ui_folder.clone().display().to_string());
            }
        });
    }

    fn advanced_settings(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        heading(
            ui,
            "Advanced",
            "Connection details for the installed Codex CLI.",
        );
        ui.label(RichText::new("Codex CLI executable").font(theme::semibold(14.0)));
        ui.add_enabled(
            !self.refreshing && self.running.is_none(),
            egui::TextEdit::singleline(&mut self.session.codex_path).desired_width(f32::INFINITY),
        );
        ui.label(RichText::new("Select the CLI executable, rather than the desktop app launcher. Bundled desktop paths are corrected automatically.").small().color(muted()));
        ui.add_enabled_ui(!self.refreshing && self.running.is_none(), |ui| {
            ui.horizontal(|ui| {
                if ui.button("Auto-detect").clicked() {
                    self.session.codex_path = codex::find_executable();
                    self.refresh_models(context);
                }
                if ui.button("Browse…").clicked() {
                    self.pick_codex_executable(context);
                }
                if ui.button("Apply & reconnect").clicked() {
                    self.refresh_models(context);
                }
            });
        });
        ui.add_space(18.0);
        ui.label(RichText::new("Connection status").font(theme::semibold(14.0)));
        ui.add(egui::Label::new(&self.status).wrap().selectable(true));
        ui.add_space(22.0);
        ui.label(format!("ReDox {}", env!("CARGO_PKG_VERSION")));
        if ui.button("Copy connection details").clicked() {
            context.copy_text(format!(
                "ReDox {}\nCLI: {}\nAccount: {}\nModel: {}\nStatus: {}",
                env!("CARGO_PKG_VERSION"),
                self.session.codex_path,
                self.account,
                self.session.model,
                self.status
            ));
        }
    }
}

fn heading(ui: &mut egui::Ui, title: &str, description: &str) {
    if !ui.visuals().dark_mode {
        ui.label(RichText::new(title).font(theme::semibold(15.0)));
        ui.label(description);
        ui.add_space(8.0);
        return;
    }
    ui.label(RichText::new(title).font(theme::semibold(24.0)));
    ui.label(RichText::new(description).color(muted()));
    ui.separator();
    ui.add_space(6.0);
}

fn settings_group(ui: &mut egui::Ui, title: &str, contents: impl FnOnce(&mut egui::Ui)) {
    if ui.visuals().dark_mode {
        theme::card(ui, |ui| {
            ui.label(RichText::new(title).font(theme::semibold(15.0)));
            contents(ui);
        });
    } else {
        win98::group_box(ui, title, contents);
    }
}

fn settings_tab(ui: &mut egui::Ui, label: &str, icon: Icon, selected: bool) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), 35.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            label,
        )
    });
    let painter = ui.painter();
    if selected || response.hovered() || response.has_focus() {
        painter.rect_filled(rect, 6, surface());
    }
    if selected {
        painter.rect_filled(
            egui::Rect::from_center_size(rect.left_center() + vec2(3.0, 0.0), vec2(2.0, 17.0)),
            1,
            theme::blue(),
        );
    }
    icons::paint(
        painter,
        egui::Rect::from_center_size(rect.left_center() + vec2(17.0, 0.0), vec2(15.0, 15.0)),
        icon,
        if selected { theme::blue() } else { muted() },
    );
    painter.text(
        rect.left_center() + vec2(31.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        theme::text(),
    );
    response
}
