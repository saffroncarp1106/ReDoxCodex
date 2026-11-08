use eframe::egui::{self, Align, Color32, Frame, Layout, RichText, vec2};
use redox::delegation::Status;

use super::{icons, theme};
use crate::app::ReDox;

impl ReDox {
    pub(in crate::app) fn agents_panel(&mut self, context: &egui::Context) {
        if !self.ui_preferences.agents_open {
            return;
        }
        let idle = self.running.is_none() && self.history_reading.is_none();
        let width = context.content_rect().width();
        let max_width = (width * 0.46).clamp(220.0, 420.0);
        egui::SidePanel::right("agents_panel")
            .default_width(320.0_f32.min(max_width))
            .width_range(270.0_f32.min(max_width)..=max_width)
            .frame(Frame::NONE.fill(theme::sidebar()).inner_margin(12))
            .show(context, |ui| {
                self.agents_header(ui);
                ui.separator();
                if !self.ui_preferences.agents_open {
                    return;
                }
                theme::panel_scroll(ui);
                egui::ScrollArea::vertical()
                    .id_salt("agent_settings")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.delegated_tasks(ui);
                        self.agent_connection(ui, context, idle);
                        self.agent_delegation(ui, context, idle);
                        self.agent_options(ui, idle);
                        self.agent_usage(ui);
                    });
            });
    }

    fn agents_header(&mut self, ui: &mut egui::Ui) {
        ui.allocate_ui_with_layout(
            vec2(ui.available_width(), 30.0),
            Layout::left_to_right(Align::Center),
            |ui| {
                let (icon, _) = ui.allocate_exact_size(vec2(24.0, 24.0), egui::Sense::hover());
                icons::paint(ui.painter(), icon, icons::Icon::Agent, theme::text());
                ui.label(RichText::new("Agents").font(theme::semibold(20.0)));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icons::button(ui, icons::Icon::Close, "Hide Agents").clicked() {
                        self.ui_preferences.agents_open = false;
                    }
                });
            },
        );
        let active = self.session.active();
        let provider = if self.is_ollama() {
            active.map_or("Ollama", |chat| chat.ollama_model.as_str())
        } else {
            "Codex / ChatGPT"
        };
        ui.add(
            egui::Label::new(RichText::new(format!("Current chat · {provider}")).small())
                .truncate(),
        )
        .on_hover_text(provider);
        if let Some(path) = active.and_then(|chat| chat.workspace.as_deref()) {
            let name = path
                .file_name()
                .unwrap_or(path.as_os_str())
                .to_string_lossy();
            ui.add(egui::Label::new(RichText::new(name).small().color(theme::muted())).truncate())
                .on_hover_text(path.display().to_string());
        }
        if self.is_ollama() && self.active_chat_running() {
            theme::status_line(ui, RichText::new(&self.status).small(), theme::blue());
        }
    }

    fn agent_connected(&self) -> bool {
        self.ollama_endpoint.as_deref() == Some(self.session.ollama.endpoint.as_str())
    }

    fn agent_connection(&mut self, ui: &mut egui::Ui, context: &egui::Context, idle: bool) {
        theme::card(ui, |ui| {
            ui.label(RichText::new("Ollama server").font(theme::semibold(15.0)));
            let editable = idle && !self.ollama_connecting;
            let endpoint = ui.add_enabled(
                editable,
                egui::TextEdit::singleline(&mut self.session.ollama.endpoint)
                    .hint_text("Server URL")
                    .desired_width(f32::INFINITY),
            );
            let enter =
                endpoint.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            let button = if self.agent_connected() {
                "Refresh models"
            } else {
                "Connect"
            };
            if ui
                .add_enabled(editable, egui::Button::new(button))
                .clicked()
                || (editable && enter)
            {
                self.refresh_ollama(context);
            }
            let connected = self.agent_connected();
            if self.ollama_connecting {
                theme::status_line(ui, "Connecting…", theme::blue());
            } else if connected {
                theme::status_line(
                    ui,
                    format!("Connected · {} models", self.ollama_models.len()),
                    theme::blue(),
                );
            } else {
                theme::status_line(ui, "Disconnected", theme::muted());
            }
            if let Some(error) = &self.ollama_error {
                ui.label(RichText::new(error).small().color(error_color(ui)));
            }
            ui.separator();
            ui.label(RichText::new("Agent model").strong());
            ui.add_enabled_ui(idle && connected && !self.ollama_models.is_empty(), |ui| {
                egui::ComboBox::from_id_salt("ollama_agent_model")
                    .width((ui.available_width() - 2.0).max(0.0))
                    .selected_text(if self.session.ollama.preferred_model.is_empty() {
                        "Choose an installed model"
                    } else {
                        &self.session.ollama.preferred_model
                    })
                    .show_ui(ui, |ui| {
                        for model in &self.ollama_models {
                            let selected = self.session.ollama.preferred_model == model.name;
                            if ui.selectable_label(selected, &model.name).clicked() {
                                self.session.ollama.preferred_model.clone_from(&model.name);
                            }
                        }
                    });
            });
            if let Some(model) = self
                .ollama_models
                .iter()
                .find(|model| model.name == self.session.ollama.preferred_model)
            {
                let details = [
                    &model.details.parameter_size,
                    &model.details.quantization_level,
                ]
                .into_iter()
                .filter(|detail| !detail.is_empty())
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(" · ");
                if !details.is_empty() {
                    ui.label(RichText::new(details).small().color(theme::muted()));
                }
            } else if connected && self.ollama_models.is_empty() {
                ui.label(
                    RichText::new("Install a model in Ollama, then refresh.")
                        .small()
                        .color(theme::muted()),
                );
            }
            ui.horizontal_wrapped(|ui| {
                ui.add_enabled_ui(
                    idle && connected && !self.session.ollama.preferred_model.is_empty(),
                    |ui| {
                        if ui.add(agent_action(ui, "Use for this chat")).clicked() {
                            self.use_ollama(false);
                        }
                        let can_new = self.can_new_chat()
                            || !self.is_ollama()
                            || self.session.active().is_some_and(|chat| {
                                chat.ollama_model != self.session.ollama.preferred_model
                            });
                        if ui
                            .add_enabled(can_new, egui::Button::new("New agent chat"))
                            .clicked()
                        {
                            self.use_ollama(true);
                        }
                    },
                );
                if self.is_ollama()
                    && ui
                        .add_enabled(idle, egui::Button::new("Switch to Codex"))
                        .clicked()
                {
                    self.use_codex();
                }
            });
        });
    }

    fn agent_delegation(&mut self, ui: &mut egui::Ui, context: &egui::Context, idle: bool) {
        theme::card(ui, |ui| {
            ui.label(RichText::new("Delegation").font(theme::semibold(15.0)));
            ui.add_enabled_ui(idle, |ui| {
                ui.checkbox(
                    &mut self.session.ollama.codex_delegation,
                    "Let Codex delegate to Ollama",
                );
            });
            if !self.session.ollama.codex_delegation {
                return;
            }
            ui.label(
                RichText::new("Codex stays in charge and reviews the agent’s work.")
                    .small()
                    .color(theme::muted()),
            );
            ui.add(
                egui::TextEdit::multiline(&mut self.ui_preferences.agent_task)
                    .hint_text("Task for Codex to hand off…")
                    .desired_rows(2)
                    .char_limit(8000)
                    .desired_width(f32::INFINITY),
            );
            let ready = idle
                && self.agent_connected()
                && !self.is_ollama()
                && !self.session.ollama.preferred_model.is_empty()
                && !self.ui_preferences.agent_task.trim().is_empty()
                && self
                    .session
                    .active()
                    .is_some_and(|chat| !chat.archived && chat.draft.trim().is_empty());
            if ui.add_enabled(ready, agent_action(ui, "Ask Codex to delegate"))
                .on_disabled_hover_text("Use a Codex chat, connect an agent, and send or clear the main chat draft first.").clicked()
            {
                self.ask_codex_to_delegate(context);
            }
        });
    }

    fn agent_options(&mut self, ui: &mut egui::Ui, idle: bool) {
        theme::card(ui, |ui| {
            if ui
                .add_enabled(
                    idle && !self.picking_workspace,
                    egui::Button::new("Choose project folder"),
                )
                .clicked()
            {
                self.pick_workspace(ui.ctx());
            }
            egui::CollapsingHeader::new("Model & tool options").show(ui, |ui| {
                ui.add_enabled_ui(idle, |ui| {
                    ui.checkbox(&mut self.session.ollama.cpu_only, "CPU only (GPU troubleshooting)");
                    ui.checkbox(&mut self.session.ollama.project_tools, "Allow project tools")
                        .on_hover_text("Requires tool support. The chat’s access mode controls file edits and commands.");
                    egui::Grid::new("ollama_token_limits").show(ui, |ui| {
                        ui.label("Context tokens");
                        ui.add(egui::DragValue::new(&mut self.session.ollama.context_tokens).range(2048..=32768).speed(256));
                        ui.end_row();
                        ui.label("Response limit");
                        ui.add(egui::DragValue::new(&mut self.session.ollama.output_tokens).range(8..=8192).speed(64));
                        ui.end_row();
                    });
                    ui.label("Agent instructions (optional)");
                    ui.add(egui::TextEdit::multiline(&mut self.session.ollama.instructions).desired_rows(3).char_limit(4000).desired_width(f32::INFINITY));
                });
            });
        });
    }

    fn agent_usage(&self, ui: &mut egui::Ui) {
        let Some(chat) = self.session.active() else {
            return;
        };
        let usage = &chat.ollama_usage;
        theme::card(ui, |ui| {
            ui.label(RichText::new("Ollama usage").font(theme::semibold(15.0)))
                .on_hover_text("Counts include delegated work in this chat.");
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("{} requests", usage.requests));
                ui.label(format!(
                    "{} input · {} output",
                    usage.prompt_tokens, usage.output_tokens
                ));
            });
            if usage.last.requests > 0 {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "Last run · {} in / {} out",
                            usage.last.prompt_tokens, usage.last.output_tokens
                        ))
                        .small()
                        .color(theme::muted()),
                    );
                    if usage.last.eval_seconds > 0.0 {
                        ui.label(
                            RichText::new(format!(
                                "{:.1} tokens/s",
                                usage.last.output_tokens as f64 / usage.last.eval_seconds
                            ))
                            .small()
                            .color(theme::muted()),
                        );
                    }
                });
            }
        });
    }

    fn delegated_tasks(&self, ui: &mut egui::Ui) {
        let Some(chat) = self
            .session
            .active()
            .filter(|chat| !chat.delegated_tasks.is_empty())
        else {
            return;
        };
        theme::card(ui, |ui| {
            ui.label(RichText::new("Delegated tasks").font(theme::semibold(15.0)));
            for task in chat.delegated_tasks.iter().rev().take(5) {
                let status = match task.status {
                    Status::Running => "Running",
                    Status::Complete => "Complete",
                    Status::Failed => "Failed",
                    Status::Cancelled => "Cancelled",
                    Status::Interrupted => "Interrupted",
                };
                let title: String = task
                    .task
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .chars()
                    .take(24)
                    .collect();
                egui::CollapsingHeader::new(format!("{status} · {title}"))
                    .id_salt(&task.id)
                    .show(ui, |ui| {
                        ui.label(RichText::new(&task.model).small().color(theme::muted()));
                        ui.label(&task.task);
                        if !task.output.is_empty() {
                            egui::ScrollArea::vertical()
                                .id_salt(("worker_output", &task.id))
                                .max_height(180.0)
                                .show(ui, |ui| {
                                    super::markdown::show_in_workspace(
                                        ui,
                                        &task.output,
                                        chat.workspace.as_deref(),
                                    );
                                });
                        }
                    });
                if task.status == Status::Running {
                    ui.add(egui::Label::new(RichText::new(&task.progress).small()).truncate())
                        .on_hover_text(&task.progress);
                }
                if let Some(error) = &task.error {
                    ui.add(
                        egui::Label::new(RichText::new(error).small().color(error_color(ui)))
                            .truncate(),
                    )
                    .on_hover_text(error);
                }
            }
        });
    }
}

fn agent_action(ui: &egui::Ui, label: &'static str) -> egui::Button<'static> {
    let button = egui::Button::new(label);
    if ui.visuals().dark_mode {
        button
            .fill(theme::blue().gamma_multiply(0.16))
            .stroke(egui::Stroke::new(1.0, theme::blue().gamma_multiply(0.7)))
    } else {
        button
    }
}

fn error_color(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::LIGHT_RED
    } else {
        Color32::DARK_RED
    }
}
