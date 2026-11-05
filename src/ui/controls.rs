use crate::app::ForgeDesk;
use eframe::egui::{self, RichText};
use forgedesk::domain::DEFAULT_MODEL;

impl ForgeDesk {
    pub(in crate::app) fn effort_picker(&mut self, ui: &mut egui::Ui, id: &str) {
        let Some(model) = self.selected_model_info().cloned() else {
            ui.add_enabled(false, egui::Button::new("Effort"))
                .on_disabled_hover_text(
                    "Refresh account and models to load supported reasoning levels.",
                );
            return;
        };
        let mut choice = self
            .session
            .model_efforts
            .get(&model.model)
            .cloned()
            .unwrap_or_default();
        let effective = model.effective_effort(Some(&choice)).unwrap_or("Default");
        let label = effort_label(effective);
        ui.add_enabled_ui(self.running.is_none() && !model.supported_reasoning_efforts.is_empty(), |ui| {
            egui::ComboBox::from_id_salt(id).width(65.0)
                .selected_text(RichText::new(label).size(12.0))
                .show_ui(ui, |ui| {
                    let default = model.default_reasoning_effort.as_deref().map(effort_label).unwrap_or_else(|| "Server default".into());
                    if ui.selectable_value(&mut choice, String::new(), format!("Model default ({default})")).clicked() { ui.close(); }
                    for option in &model.supported_reasoning_efforts {
                        if ui.selectable_value(&mut choice, option.reasoning_effort.clone(), effort_label(&option.reasoning_effort))
                            .on_hover_text(&option.description).clicked() { ui.close(); }
                    }
                }).response.on_hover_text("Reasoning effort for the next message. Remembered separately for each model.");
        });
        if choice.is_empty() {
            self.session.model_efforts.remove(&model.model);
        } else {
            self.session.model_efforts.insert(model.model, choice);
        }
    }
}

fn effort_label(effort: &str) -> String {
    match effort {
        "xhigh" => "Extra high".into(),
        "none" => "None".into(),
        value => {
            let mut chars = value.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        }
    }
}

pub(super) fn model_label(model: &str) -> String {
    if model == DEFAULT_MODEL {
        return model.into();
    }
    model
        .split('-')
        .enumerate()
        .map(|(index, word)| {
            if index == 0 {
                word.to_uppercase()
            } else {
                let mut chars = word.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_uppercase().collect::<String>() + chars.as_str()
                })
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
