use crate::app::ReDox;
use eframe::egui;
use redox_studio::{actions::UiAction, document::Document, presentation::interactive};

#[derive(Clone)]
struct LiveSkin {
    document: Result<std::sync::Arc<Document>, String>,
    stamp: Option<(std::time::SystemTime, u64)>,
    checked: std::time::Instant,
    warning: Option<String>,
}

fn stamp(path: &std::path::Path) -> Option<(std::time::SystemTime, u64)> {
    let metadata = std::fs::metadata(path).ok()?;
    Some((metadata.modified().ok()?, metadata.len()))
}

impl LiveSkin {
    fn load(path: &std::path::Path) -> Self {
        Self {
            document: Document::load(path).map(std::sync::Arc::new),
            stamp: stamp(path),
            checked: std::time::Instant::now(),
            warning: None,
        }
    }
    fn refresh(&mut self, path: &std::path::Path, force: bool) {
        if !force && self.checked.elapsed() < std::time::Duration::from_millis(500) {
            return;
        }
        self.checked = std::time::Instant::now();
        let updated = stamp(path);
        if !force && self.stamp == updated {
            return;
        }
        match Document::load(path) {
            Ok(document) => {
                self.document = Ok(std::sync::Arc::new(document));
                self.stamp = updated;
                self.warning = None;
            }
            Err(error) => {
                self.warning = Some(if self.document.is_ok() {
                    format!("Live reload: {error}. Keeping the last valid design.")
                } else {
                    format!("Live reload: {error}")
                })
            }
        }
    }
}

fn conversation(
    ui: &mut egui::Ui,
    control: &interactive::Control<'_>,
    chat: &redox::domain::Chat,
    codex_mode: bool,
) {
    interactive::content(ui, control, |ui| {
        let mut palette = super::theme::palette();
        if control.node.fill[3] > 0 {
            palette.background = egui::Color32::from_rgba_unmultiplied(
                control.node.fill[0],
                control.node.fill[1],
                control.node.fill[2],
                control.node.fill[3],
            );
        }
        palette.text = control.text_color;
        palette.surface = if ui.visuals().dark_mode {
            egui::Color32::from_gray(42)
        } else {
            egui::Color32::from_gray(240)
        };
        palette.input_background = palette.surface;
        super::theme::with_palette(palette, || {
            if chat.messages.is_empty() {
                ui.label(&control.node.text);
            } else {
                super::conversation::render_transcript(
                    ui,
                    chat,
                    ("skin_conversation", control.response.id, chat.id),
                    0.,
                    codex_mode,
                );
            }
        });
    });
}

impl ReDox {
    pub(in crate::app) fn load_studio_skin(&mut self, path: std::path::PathBuf) {
        match Document::load(&path) {
            Ok(_) => {
                self.ui_preferences.studio_skin = Some(path);
                self.ui_error = None;
            }
            Err(error) => {
                self.ui_error = Some(error);
                self.settings_page = super::SettingsPage::Appearance;
                self.settings_open = true;
            }
        }
    }

    pub(super) fn studio_skin_settings(&mut self, ui: &mut egui::Ui) {
        ui.label("Studio controls (.redoxskin)");
        ui.horizontal(|ui| {
            if ui.button("Load Studio controls…").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Studio skin", &["redoxskin"])
                    .pick_file()
            {
                self.load_studio_skin(path);
            }
            if ui.button("Remove Studio controls").clicked() {
                self.ui_preferences.studio_skin = None;
            }
        });
        ui.label(
            "Available in every theme. Studio saves reload automatically while ReDox is running.",
        );
    }

    fn studio_model_choices(&mut self, ui: &mut egui::Ui) {
        ui.add_enabled_ui(self.running.is_none(), |ui| {
            if self.is_ollama() {
                let models = self
                    .ollama_models
                    .iter()
                    .map(|m| m.name.clone())
                    .collect::<Vec<_>>();
                if let Some(chat) = self.session.active_mut() {
                    for model in models {
                        if ui
                            .selectable_value(&mut chat.ollama_model, model.clone(), model)
                            .clicked()
                        {
                            ui.close();
                        }
                    }
                }
            } else {
                let mut models = vec![redox::domain::DEFAULT_MODEL.to_owned()];
                models.extend(
                    self.models
                        .iter()
                        .filter(|name| name.as_str() != redox::domain::DEFAULT_MODEL)
                        .cloned(),
                );
                for model in models {
                    if ui
                        .selectable_value(
                            &mut self.session.model,
                            model.clone(),
                            super::controls::model_label(&model),
                        )
                        .clicked()
                    {
                        ui.close();
                    }
                }
            }
        });
    }

    fn studio_effort_choices(&mut self, ui: &mut egui::Ui) {
        if self.is_ollama() {
            ui.label("Ollama model settings");
            return;
        }
        let Some(model) = self.selected_model_info().cloned() else {
            ui.label("Refresh models to load reasoning levels");
            return;
        };
        ui.add_enabled_ui(self.running.is_none(), |ui| {
            if ui
                .selectable_label(
                    !self.session.model_efforts.contains_key(&model.model),
                    "Model default",
                )
                .clicked()
            {
                self.session.model_efforts.remove(&model.model);
                ui.close();
            }
            for option in &model.supported_reasoning_efforts {
                let selected =
                    self.session.model_efforts.get(&model.model) == Some(&option.reasoning_effort);
                if ui
                    .selectable_label(selected, &option.reasoning_effort)
                    .on_hover_text(&option.description)
                    .clicked()
                {
                    self.session
                        .model_efforts
                        .insert(model.model.clone(), option.reasoning_effort.clone());
                    ui.close();
                }
            }
        });
    }

    pub(super) fn studio_controls(&mut self, ctx: &egui::Context) {
        let Some(path) = self.ui_preferences.studio_skin.clone() else {
            return;
        };
        let key = egui::Id::new(("studio_document", &path));
        let mut cached = ctx
            .data_mut(|d| d.get_temp::<LiveSkin>(key))
            .unwrap_or_else(|| LiveSkin::load(&path));
        cached.refresh(&path, false);
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
        let mut calls = vec![];
        let mut submitted = false;
        let mut open = true;
        egui::Window::new("Studio controls")
            .id(egui::Id::new("studio_controls"))
            .open(&mut open)
            .default_width(720.)
            .show(ctx, |ui| {
                if ui.button("Reload from Studio").clicked() {
                    cached.refresh(&path, true);
                }
                if let Some(warning) = &cached.warning {
                    ui.colored_label(egui::Color32::LIGHT_RED, warning);
                }
                match &cached.document {
                    Ok(document) => {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let model = if self.is_ollama() {
                                self.session
                                    .active()
                                    .map(|chat| chat.ollama_model.clone())
                                    .unwrap_or_default()
                            } else {
                                super::controls::model_label(&self.session.model)
                            };
                            let effort = self.selected_effort().unwrap_or_else(|| "Default".into());
                            let combined = format!("{} / {}", model, effort);
                            calls = interactive::show_with_values(
                                ui,
                                document,
                                interactive::Values {
                                    model: Some(&model),
                                    effort: Some(&effort),
                                    model_effort: Some(&combined),
                                },
                                &mut |ui, control| match control.action {
                                    UiAction::Composer => {
                                        if let Some(chat) = self.session.active_mut() {
                                            let result = interactive::composer(
                                                ui,
                                                control,
                                                &mut chat.draft,
                                                egui::Id::new(chat.id),
                                                false,
                                            );
                                            if result.submitted {
                                                submitted = true;
                                            }
                                        }
                                    }
                                    UiAction::Conversation => {
                                        if let Some(chat) = self.session.active() {
                                            conversation(
                                                ui,
                                                control,
                                                chat,
                                                self.ui_preferences.codex_mode,
                                            );
                                        }
                                    }
                                    UiAction::Model | UiAction::ModelEffort => {
                                        egui::Popup::menu(&control.response).width(220.).show(
                                            |ui| {
                                                self.studio_model_choices(ui);
                                                if control.action == UiAction::ModelEffort {
                                                    ui.separator();
                                                    self.studio_effort_choices(ui);
                                                }
                                            },
                                        );
                                    }
                                    UiAction::Effort => {
                                        egui::Popup::menu(&control.response)
                                            .width(200.)
                                            .show(|ui| self.studio_effort_choices(ui));
                                    }
                                    UiAction::Menus => {
                                        egui::Popup::menu(&control.response)
                                            .show(|ui| self.main_menus(ui, ctx));
                                    }
                                    _ => {}
                                },
                            );
                        });
                    }
                    Err(error) => {
                        ui.colored_label(egui::Color32::LIGHT_RED, error);
                    }
                }
            });
        ctx.data_mut(|d| d.insert_temp(key, cached));
        if !open {
            self.ui_preferences.studio_skin = None;
        }
        if submitted {
            calls.push(UiAction::Send);
        }
        for action in calls {
            self.dispatch_ui_action(action, ctx);
        }
    }
}

#[cfg(test)]
#[path = "../../tests/ui/studio.rs"]
mod tests;
