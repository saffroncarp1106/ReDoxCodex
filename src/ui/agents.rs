use eframe::egui::{self, Frame, Margin, RichText};
use forgedesk::domain::Provider;

use super::theme;
use crate::app::ForgeDesk;

impl ForgeDesk {
    pub(in crate::app) fn agents_panel(&mut self, context: &egui::Context) {
        if !self.ui_preferences.agents_open {
            egui::SidePanel::right("agents_tab")
                .exact_width(88.0)
                .resizable(false)
                .frame(
                    Frame::NONE
                        .fill(theme::rail())
                        .inner_margin(Margin::symmetric(6, 12)),
                )
                .show(context, |ui| {
                    if ui
                        .add_sized(
                            [ui.available_width(), 30.0],
                            egui::Button::new("Agents").wrap_mode(egui::TextWrapMode::Extend),
                        )
                        .on_hover_text("Connect an Ollama agent")
                        .clicked()
                    {
                        self.ui_preferences.agents_open = true;
                    }
                    if self.is_ollama() {
                        ui.label(RichText::new("Local").small().color(theme::blue()));
                    }
                });
            return;
        }
        egui::SidePanel::right("agents_panel").default_width(300.0).width_range(270.0..=420.0)
            .frame(Frame::NONE.fill(theme::sidebar()).inner_margin(14))
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Agents");
                    if ui.small_button("Hide").clicked() { self.ui_preferences.agents_open = false; }
                });
                ui.label(RichText::new("Ollama").strong());
                ui.label("Choose a model to handle this chat and project requests.");
                if context.viewport_rect().width() < 1280.0 {
                    ui.label(RichText::new("Hide Agents to return to the chat sidebar.").small().color(theme::muted()));
                }
                ui.add_space(8.0);
                ui.spacing_mut().scroll = theme::conversation_scroll();
                egui::ScrollArea::vertical().id_salt("agent_settings").show(ui, |ui| {
                    if self.session.active().is_some_and(|chat| !chat.delegated_tasks.is_empty()) {
                        self.delegated_tasks(ui);
                    }
                    let idle = self.running.is_none() && self.history_reading.is_none();
                    ui.label("Server URL");
                    ui.add_enabled(idle && !self.ollama_connecting,
                        egui::TextEdit::singleline(&mut self.session.ollama.endpoint).desired_width(f32::INFINITY));
                    if ui.add_enabled(idle && !self.ollama_connecting, egui::Button::new("Connect / refresh models")).clicked() {
                        self.refresh_ollama(context);
                    }
                    if self.ollama_connecting { ui.spinner(); ui.label("Connecting to Ollama…"); }
                    if let Some(error) = &self.ollama_error { ui.colored_label(egui::Color32::LIGHT_RED, error); }
                    let connected = self.ollama_endpoint.as_deref() == Some(self.session.ollama.endpoint.as_str());
                    if connected {
                        ui.label(RichText::new(format!("Connected · {} installed models", self.ollama_models.len())).color(theme::blue()));
                    }
                    ui.add_space(12.0);
                    ui.label("Agent model");
                    ui.add_enabled_ui(idle && connected, |ui| {
                        egui::ComboBox::from_id_salt("ollama_agent_model").width((ui.available_width() - 8.0).max(120.0))
                            .selected_text(if self.session.ollama.preferred_model.is_empty() { "Choose an installed model" } else { &self.session.ollama.preferred_model })
                            .show_ui(ui, |ui| {
                                for model in &self.ollama_models {
                                    ui.selectable_value(&mut self.session.ollama.preferred_model, model.name.clone(), &model.name);
                                }
                            });
                    });
                    if let Some(model) = self.ollama_models.iter().find(|model| model.name == self.session.ollama.preferred_model) {
                        ui.label(RichText::new(format!("{} · {}", model.details.parameter_size, model.details.quantization_level)).small().color(theme::muted()));
                    }
                    if connected && self.ollama_models.is_empty() {
                        ui.label("No models installed. Install a model in Ollama, then refresh.");
                    }
                    ui.add_space(8.0);
                    ui.add_enabled_ui(idle, |ui| {
                        ui.checkbox(&mut self.session.ollama.codex_delegation, "Let Codex delegate to this agent");
                    });
                    ui.label(RichText::new("Codex stays in the main chat, sends tasks to the selected model, and reviews its results.").small().color(theme::muted()));
                    ui.add(egui::TextEdit::multiline(&mut self.ui_preferences.agent_task)
                        .hint_text("Task for Codex to hand off…").desired_rows(2).char_limit(8000).desired_width(f32::INFINITY));
                    let ready = idle && connected && !self.is_ollama() && self.session.ollama.codex_delegation
                        && !self.session.ollama.preferred_model.is_empty() && !self.ui_preferences.agent_task.trim().is_empty()
                        && self.session.active().is_some_and(|chat| !chat.archived && chat.draft.trim().is_empty());
                    if ui.add_enabled(ready, egui::Button::new("Ask Codex to delegate")).on_disabled_hover_text("Use a Codex chat, connect an agent, and send or clear the main chat draft first.").clicked() {
                        self.ask_codex_to_delegate(context);
                    }
                    ui.add_space(8.0);
                    ui.add_enabled_ui(idle && connected && !self.session.ollama.preferred_model.is_empty(), |ui| {
                        if ui.button("Use for this chat").clicked() { self.use_ollama(false); }
                        let can_new = self.can_new_chat() || !self.is_ollama() || self.session.active().is_some_and(|chat| chat.ollama_model != self.session.ollama.preferred_model);
                        if ui.add_enabled(can_new, egui::Button::new("New agent chat")).clicked() { self.use_ollama(true); }
                    });
                    if self.is_ollama() && ui.add_enabled(idle, egui::Button::new("Switch this chat to Codex")).clicked() { self.use_codex(); }
                    ui.separator();
                    if let Some(chat) = self.session.active() {
                        ui.label(RichText::new(if chat.provider == Provider::Ollama { format!("Active: Ollama · {}", chat.ollama_model) } else { "Active: Codex / ChatGPT".into() }).strong());
                        if let Some(path) = &chat.workspace { ui.label(RichText::new(path.display().to_string()).small()); }
                    }
                    if ui.add_enabled(idle && !self.picking_workspace, egui::Button::new("Choose project folder")).clicked() { self.pick_workspace(context); }
                    ui.add_enabled_ui(idle, |ui| {
                        ui.checkbox(&mut self.session.ollama.cpu_only, "CPU only (GPU troubleshooting)");
                        ui.checkbox(&mut self.session.ollama.project_tools, "Allow project tools");
                        ui.label(RichText::new("Requires a model with tool support. Read-only mode cannot edit files or run commands. Workspace mode asks before edits and commands. Commands run with your OS permissions.").small().color(theme::muted()));
                        ui.horizontal(|ui| {
                            ui.label("Context tokens");
                            ui.add(egui::DragValue::new(&mut self.session.ollama.context_tokens).range(2048..=32768).speed(256));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Response limit");
                            ui.add(egui::DragValue::new(&mut self.session.ollama.output_tokens).range(8..=8192).speed(64));
                        });
                        ui.label("Agent instructions (optional)");
                        ui.add(egui::TextEdit::multiline(&mut self.session.ollama.instructions).desired_rows(3).char_limit(4000).desired_width(f32::INFINITY));
                    });
                    ui.separator();
                    ui.label(RichText::new("Ollama usage · includes delegated work").strong());
                    if let Some(chat) = self.session.active() {
                        let usage = &chat.ollama_usage;
                        ui.label(format!("This chat: {} input · {} output tokens", usage.prompt_tokens, usage.output_tokens));
                        ui.label(format!("{} Ollama requests", usage.requests));
                        if usage.last.requests > 0 {
                            ui.label(format!("Last run: {} input · {} output", usage.last.prompt_tokens, usage.last.output_tokens));
                            if usage.last.eval_seconds > 0.0 { ui.label(format!("{:.1} tokens/s", usage.last.output_tokens as f64 / usage.last.eval_seconds)); }
                        }
                    }
                    if self.is_ollama() && self.running.is_some() { ui.spinner(); ui.label(&self.status); }
                    ui.label(RichText::new("Ollama worker requests go directly to your server. Codex orchestration still uses your Codex account quota. No models are downloaded automatically.").small().color(theme::muted()));
                });
            });
    }

    fn delegated_tasks(&self, ui: &mut egui::Ui) {
        let Some(chat) = self.session.active() else {
            return;
        };
        ui.label(RichText::new("Delegated tasks").strong());
        if chat.delegated_tasks.is_empty() {
            ui.label(
                RichText::new("Tasks Codex sends to Ollama will appear here.")
                    .small()
                    .color(theme::muted()),
            );
        }
        for (index, task) in chat.delegated_tasks.iter().rev().take(5).enumerate() {
            let status = match task.status {
                forgedesk::delegation::Status::Running => "Running",
                forgedesk::delegation::Status::Complete => "Complete",
                forgedesk::delegation::Status::Failed => "Failed",
                forgedesk::delegation::Status::Cancelled => "Cancelled",
                forgedesk::delegation::Status::Interrupted => "Interrupted",
            };
            egui::CollapsingHeader::new(format!("{} · {status}", task.model))
                .id_salt(&task.id)
                .default_open(index == 0 || task.status == forgedesk::delegation::Status::Running)
                .show(ui, |ui| {
                    ui.label(&task.task);
                    if task.status == forgedesk::delegation::Status::Running {
                        ui.spinner();
                        ui.label(&task.progress);
                    }
                    if !task.output.is_empty() {
                        egui::ScrollArea::vertical()
                            .id_salt(("worker_output", &task.id))
                            .max_height(200.0)
                            .show(ui, |ui| {
                                super::markdown::show_in_workspace(
                                    ui,
                                    &task.output,
                                    chat.workspace.as_deref(),
                                );
                            });
                    }
                    if let Some(error) = &task.error {
                        ui.colored_label(egui::Color32::LIGHT_RED, error);
                    }
                });
        }
        ui.separator();
    }
}
