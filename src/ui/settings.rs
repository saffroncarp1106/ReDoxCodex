use eframe::egui::{self, Align, Color32, Frame, Layout, RichText, vec2};

use super::controls::model_label;
use super::icons::{self, Icon};
use super::theme::{muted, surface};
use super::{Preferences, SettingsPage, theme};
use crate::app::ForgeDesk;
use forgedesk::codex;
use forgedesk::domain::{DEFAULT_MODEL, Mode};

impl ForgeDesk {
    pub(in crate::app) fn settings(&mut self, context: &egui::Context) {
        let mut open = self.settings_open;
        let animation_was_active = self.startup.is_active();
        let available = context.content_rect().size();
        let size = vec2(
            (available.x - 40.0).clamp(480.0, 740.0),
            (available.y - 90.0).clamp(280.0, 520.0),
        );
        egui::Window::new("Settings")
            .id(egui::Id::new("settings_panel_v3"))
            .default_pos(context.content_rect().center() - size * 0.5)
            .open(&mut open)
            .fixed_size(size)
            .resizable(false)
            .collapsible(false)
            .show(context, |ui| {
                ui.add_space(8.0);
                let pages = [
                    (SettingsPage::Account, "Account"),
                    (SettingsPage::Models, "Models & reasoning"),
                    (SettingsPage::Workspace, "Workspace & access"),
                    (SettingsPage::Appearance, "Appearance"),
                    (SettingsPage::Cursors, "Cursor animation"),
                    (SettingsPage::Advanced, "Advanced"),
                ];
                if ui.available_width() < 620.0 {
                    let title = pages
                        .iter()
                        .find(|(page, _)| *page == self.settings_page)
                        .map_or("Account", |(_, title)| *title);
                    egui::ComboBox::from_id_salt("settings_section")
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
                        ui.allocate_ui_with_layout(
                            vec2(150.0, (ui.available_height() - 8.0).max(200.0)),
                            Layout::top_down(Align::Min),
                            |ui| {
                                ui.spacing_mut().item_spacing.y = 6.0;
                                for (page, label) in pages {
                                    let selected = self.settings_page == page;
                                    let button = egui::Button::new(label)
                                        .fill(if selected {
                                            surface()
                                        } else {
                                            Color32::TRANSPARENT
                                        })
                                        .corner_radius(8);
                                    if ui.add_sized([145.0, 36.0], button).clicked() {
                                        self.settings_page = page;
                                    }
                                }
                            },
                        );
                        ui.separator();
                        self.settings_content(ui, context);
                    });
                }
            });
        self.settings_open = open && (animation_was_active || !self.startup.is_active());
    }

    fn settings_content(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        ui.spacing_mut().scroll = theme::conversation_scroll();
        egui::ScrollArea::vertical()
            .id_salt("settings_content")
            .max_height((ui.available_height() - 16.0).max(220.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_min_width((ui.available_width() - 8.0).max(260.0));
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
        Frame::new()
            .fill(surface())
            .corner_radius(10)
            .inner_margin(14)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    let color = if self.signed_in {
                        Color32::from_rgb(74, 188, 123)
                    } else {
                        muted()
                    };
                    let (rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 4.0, color);
                    ui.label(
                        RichText::new(if self.signed_in {
                            "Connected"
                        } else if self.refreshing {
                            "Connecting…"
                        } else {
                            "Not connected"
                        })
                        .font(theme::semibold(15.0)),
                    );
                });
                ui.label(&self.account);
            });
        ui.add_space(12.0);
        if ui
            .add_enabled(!self.refreshing, egui::Button::new("Refresh connection"))
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
            .add_enabled(!signing_in, egui::Button::new(label))
            .clicked()
        {
            self.start_login(context);
        }
        if !self.login_text.is_empty() {
            ui.label(&self.login_text);
        }
        if let Some(challenge) = &self.login_challenge {
            Frame::new()
                .fill(surface())
                .corner_radius(10)
                .inner_margin(14)
                .show(ui, |ui| {
                    ui.label(RichText::new("Verification code").color(muted()));
                    ui.add(
                        egui::Label::new(
                            RichText::new(&challenge.user_code).monospace().size(25.0),
                        )
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
        ui.label(RichText::new("Model").font(theme::semibold(14.0)));
        egui::ComboBox::from_id_salt("settings_model")
            .width(300.0)
            .selected_text(model_label(&self.session.model))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.session.model, DEFAULT_MODEL.into(), DEFAULT_MODEL);
                for model in &self.models {
                    ui.selectable_value(&mut self.session.model, model.clone(), model_label(model));
                }
            });
        ui.add_space(18.0);
        ui.label(RichText::new("Reasoning effort").font(theme::semibold(14.0)));
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
        ui.add_space(18.0);
        ui.label("You can also change the model and effort directly in the composer.");
    }

    fn workspace_settings(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        heading(
            ui,
            "Workspace & access",
            "Choose a project and what the current chat can do.",
        );
        ui.label(RichText::new("Project folder").font(theme::semibold(14.0)));
        if let Some(path) = self
            .session
            .active()
            .and_then(|chat| chat.workspace.as_ref())
            .or(self.session.workspace.as_ref())
        {
            ui.add(
                egui::Label::new(RichText::new(path.display().to_string()).monospace())
                    .wrap()
                    .selectable(true),
            );
        } else {
            ui.label(RichText::new("No project selected").color(muted()));
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
            RichText::new("Opening a project creates or selects a chat for that folder.")
                .small()
                .color(muted()),
        );
        ui.add_space(20.0);
        ui.label(RichText::new("Current chat permissions").font(theme::semibold(14.0)));
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
        self.ui_picker(ui, "settings_ui_picker");
        ui.label(
            RichText::new("Drop a UI JSON file or pack folder into the window to import it.")
                .small()
                .color(muted()),
        );
        if let Some(error) = &self.ui_error {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
        for error in &self.ui_catalog.errors {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
        ui.add_space(12.0);
        ui.label(RichText::new("Interface scale").font(theme::semibold(14.0)));
        let mut percent = self.ui_preferences.zoom * 100.0;
        let changed = ui
            .scope(|ui| {
                ui.spacing_mut().slider_width = 190.0;
                let visuals = &mut ui.style_mut().visuals;
                visuals.widgets.inactive.bg_fill = theme::blue();
                visuals.widgets.inactive.weak_bg_fill = Color32::from_gray(75);
                visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, theme::blue());
                visuals.widgets.hovered.bg_fill = theme::blue();
                visuals.widgets.active.bg_fill = theme::blue();
                ui.add(
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
        ui.add_space(16.0);
        if ui.button("Restore default appearance").clicked() {
            self.ui_preferences.zoom = Preferences::default().zoom;
            self.sidebar_visible = true;
            context.set_zoom_factor(1.0);
        }
        ui.add_space(25.0);
        ui.label(RichText::new("Custom interfaces").font(theme::semibold(14.0)));
        ui.label(RichText::new("Drop JSON configs or folders containing ui.json into src/ui/layouts/. Available packs appear in the UI menu automatically. Config edits apply after selecting the UI and restarting.").color(muted()));
        if ui.button("Copy UI packs folder").clicked() {
            context.copy_text(self.ui_folder.clone().display().to_string());
        }
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
        ui.label(format!("Codex Rust {}", env!("CARGO_PKG_VERSION")));
        if ui.button("Copy connection details").clicked() {
            context.copy_text(format!(
                "Codex Rust {}\nCLI: {}\nAccount: {}\nModel: {}\nStatus: {}",
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
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(21.0, 26.0), egui::Sense::hover());
        icons::paint(
            ui.painter(),
            rect.shrink2(vec2(1.0, 3.0)),
            Icon::Settings,
            muted(),
        );
        ui.label(RichText::new(title).font(theme::semibold(21.0)));
    });
    ui.label(RichText::new(description).color(muted()));
    ui.add_space(22.0);
}
