use crate::app::ForgeDesk;
use eframe::egui;

impl ForgeDesk {
    #[cfg(debug_assertions)]
    pub(in crate::app) fn verify_ui_restart(&mut self, context: &egui::Context) {
        let Some(directory) = std::env::var_os("CODEX_RUST_VERIFY_UI_RESTART") else {
            return;
        };
        if self.startup.is_active() || context.cumulative_frame_nr() < 10 {
            context.request_repaint();
            return;
        }
        if self.active_ui.id == "codex" {
            self.session.new_chat();
            if let Some(chat) = self.session.active_mut() {
                chat.draft = "UI_RESTART_DRAFT_CHECK".into();
            }
            self.switch_ui("gemini", context);
        } else {
            let preserved = self
                .session
                .active()
                .is_some_and(|chat| chat.draft == "UI_RESTART_DRAFT_CHECK");
            let result = serde_json::json!({"ui":self.active_ui.id,"draft_preserved":preserved,"animation_completed":!self.startup.is_active()});
            let path = std::path::PathBuf::from(directory).join("result.json");
            if let Err(error) = std::fs::write(path, result.to_string()) {
                eprintln!("UI restart check: {error}");
            }
            context.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    pub(in crate::app) fn capture_frame(&mut self, context: &egui::Context) {
        let Some(path) = &self.capture_path else {
            return;
        };
        self.capture_frames += 1;
        if self.capture_frames == 1 {
            if std::env::args().any(|arg| arg == "--agents") {
                self.ui_preferences.agents_open = true;
            }
            if std::env::args().any(|arg| arg == "--format-preview") {
                self.session.new_chat();
                if let Some(chat) = self.session.active_mut() {
                    chat.title = "Equation and code formatting".into();
                    for (role, content) in [
                        (
                            forgedesk::domain::Role::User,
                            r"Calculate the pH when \([\mathrm{H}^{+}]=1.0\times10^{-3}\,\mathrm{mol/L}\).",
                        ),
                        (
                            forgedesk::domain::Role::Assistant,
                            include_str!("previews/formatting.md"),
                        ),
                    ] {
                        chat.messages.push(forgedesk::domain::Message {
                            id: uuid::Uuid::new_v4(),
                            role,
                            content: content.into(),
                            status: forgedesk::domain::MessageStatus::Complete,
                            error: None,
                            model: "Formatting preview".into(),
                            attachments: Vec::new(),
                        });
                    }
                }
            }

            if let Some(path) = std::env::args().find_map(|arg| {
                arg.strip_prefix("--media-preview=")
                    .map(std::path::PathBuf::from)
            }) {
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        self.session.new_chat();
                        if let Some(chat) = self.session.active_mut() {
                            chat.title = "Media preview".into();
                            chat.workspace = path
                                .canonicalize()
                                .ok()
                                .and_then(|path| path.parent().map(std::path::Path::to_path_buf));
                            chat.messages.push(forgedesk::domain::Message {
                                id: uuid::Uuid::new_v4(),
                                role: forgedesk::domain::Role::Assistant,
                                content,
                                status: forgedesk::domain::MessageStatus::Complete,
                                error: None,
                                model: "Local media preview".into(),
                                attachments: Vec::new(),
                            });
                        }
                    }
                    Err(error) => {
                        eprintln!("Could not read media preview {}: {error}", path.display())
                    }
                }
            }
            if std::env::args().any(|arg| arg == "--home-preview") {
                self.session.new_chat();
            }
            context.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
            context.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            context.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(956.0, 900.0)));
            context.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(80.0, 60.0)));
            if let Some(page) =
                std::env::args().find_map(|arg| arg.strip_prefix("--settings=").map(str::to_owned))
            {
                self.settings_open = true;
                self.settings_page = match page.as_str() {
                    "models" => super::SettingsPage::Models,
                    "workspace" => super::SettingsPage::Workspace,
                    "appearance" => super::SettingsPage::Appearance,
                    "cursors" => super::SettingsPage::Cursors,
                    "advanced" => super::SettingsPage::Advanced,
                    _ => super::SettingsPage::Account,
                };
            }
            if std::env::args().any(|arg| arg == "--usage") {
                self.usage_open = true;
            }
        }
        if self.capture_frames == 4
            && (self.refreshing
                || self.ollama_connecting
                || self.history_loading
                || self.history_reading.is_some()
                || (self.startup.is_active()
                    && !std::env::args().any(|arg| arg.starts_with("--startup-preview="))))
        {
            self.capture_frames = 3;
        }
        if self.capture_frames == 8 {
            context.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let screenshot = context.input(|input| {
            input.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(screenshot) = screenshot {
            let pixels: Vec<u8> = screenshot
                .pixels
                .iter()
                .flat_map(|pixel| pixel.to_array())
                .collect();
            if let Err(error) = image::save_buffer(
                path,
                &pixels,
                screenshot.width() as u32,
                screenshot.height() as u32,
                image::ColorType::Rgba8,
            ) {
                eprintln!("Could not save UI screenshot: {error}");
            }
            context.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        context.request_repaint();
    }
}
