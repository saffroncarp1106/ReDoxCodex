use super::theme;
use crate::app::ForgeDesk;
use eframe::egui::{self, Align, Layout, RichText, Sense, pos2, vec2};
use forgedesk::domain::DEFAULT_MODEL;

pub(super) fn focus_new_prompt(response: &egui::Response, pending: &mut bool, allowed: bool) {
    if *pending && allowed {
        response.request_focus();
        *pending = false;
    }
}

impl ForgeDesk {
    pub(in crate::app) fn model_effort_picker(&mut self, ui: &mut egui::Ui, id: &str) {
        let local = self.is_ollama();
        let model = self.selected_model_info().cloned();
        let choice = model
            .as_ref()
            .and_then(|model| self.session.model_efforts.get(&model.model))
            .cloned();
        let effort = model
            .as_ref()
            .and_then(|model| model.effective_effort(choice.as_deref()));
        let label = if local {
            "Local model".into()
        } else {
            effort_label(effort.unwrap_or("Default"))
        };
        let name = if local {
            self.session
                .active()
                .map(|chat| chat.ollama_model.clone())
                .unwrap_or_default()
        } else {
            model_label(&self.session.model)
        };
        ui.push_id(id, |ui| {
            ui.add_enabled_ui(self.running.is_none(), |ui| {
                ui.scope(|ui| {
                    ui.style_mut().visuals.menu_corner_radius = egui::CornerRadius::same(18);
                    let button = ui.button(RichText::new(format!("{name} · {label}")).size(12.0));
                    if self.capture_path.is_some()
                        && std::env::args().any(|arg| arg == "--effort-preview")
                    {
                        egui::Popup::open_id(ui.ctx(), egui::Popup::default_response_id(&button));
                    }
                    egui::Popup::menu(&button)
                        .align(egui::emath::RectAlign::TOP)
                        .align_alternatives(&[egui::emath::RectAlign::BOTTOM])
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .frame(
                            egui::Frame::popup(ui.style())
                                .corner_radius(18)
                                .inner_margin(12),
                        )
                        .show(|ui| {
                            ui.set_width(230.0);
                            ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                ui.allocate_ui_with_layout(
                                    vec2(22.0, 24.0),
                                    Layout::left_to_right(Align::Center),
                                    |ui| {
                                        let (rect, _) = ui
                                            .allocate_exact_size(vec2(16.0, 18.0), Sense::hover());
                                        super::icons::paint(
                                            ui.painter(),
                                            rect,
                                            super::icons::Icon::Bolt,
                                            theme::muted(),
                                        );
                                    },
                                );
                                ui.allocate_ui_with_layout(
                                    vec2(ui.available_width() - 32.0, 24.0),
                                    Layout::top_down(Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(&label)
                                                .size(14.0)
                                                .strong()
                                                .color(theme::blue()),
                                        );
                                    },
                                );
                                let can_reset = !local && choice.is_some();
                                if ui
                                    .add_enabled(can_reset, egui::Button::new("↺").frame(false))
                                    .on_hover_text("Restore this model's default effort")
                                    .clicked()
                                    && let Some(model) = &model
                                {
                                    self.session.model_efforts.remove(&model.model);
                                }
                            });
                            self.chat_model_picker(ui, "popover_model");
                            ui.add_space(3.0);
                            if !local {
                                if let Some(model) = &model {
                                    let options = &model.supported_reasoning_efforts;
                                    let mut index = options
                                        .iter()
                                        .position(|option| {
                                            Some(option.reasoning_effort.as_str()) == effort
                                        })
                                        .unwrap_or(0);
                                    let labels: Vec<_> = options
                                        .iter()
                                        .map(|option| effort_label(&option.reasoning_effort))
                                        .collect();
                                    let slider = effort_steps(ui, &labels, &mut index);
                                    if slider.changed() {
                                        self.session.model_efforts.insert(
                                            model.model.clone(),
                                            options[index].reasoning_effort.clone(),
                                        );
                                    }
                                    if let Some(option) = options.get(index) {
                                        slider.on_hover_text(&option.description);
                                    }
                                } else {
                                    ui.label(
                                        RichText::new("Refresh models to load reasoning levels.")
                                            .small()
                                            .color(theme::muted()),
                                    );
                                }
                            }
                            ui.add_space(2.0);
                        });
                });
            });
        });
    }

    pub(in crate::app) fn effort_picker(&mut self, ui: &mut egui::Ui, id: &str) {
        if self.is_ollama() {
            ui.label(RichText::new("Ollama").size(12.0));
            return;
        }
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

    pub(in crate::app) fn chat_model_picker(&mut self, ui: &mut egui::Ui, id: &str) {
        let local = self.is_ollama();
        ui.add_enabled_ui(self.running.is_none(), |ui| {
            if local {
                let options: Vec<_> = self
                    .ollama_models
                    .iter()
                    .map(|model| (model.name.clone(), model.name.clone()))
                    .collect();
                if let Some(chat) = self.session.active_mut() {
                    model_choices(ui, id, &mut chat.ollama_model, &options);
                }
            } else {
                let mut options = vec![(DEFAULT_MODEL.into(), DEFAULT_MODEL.into())];
                options.extend(
                    self.models
                        .iter()
                        .filter(|name| name.as_str() != DEFAULT_MODEL)
                        .map(|name| (name.clone(), model_label(name))),
                );
                model_choices(ui, id, &mut self.session.model, &options);
            }
        });
    }
}

fn model_choices(
    ui: &mut egui::Ui,
    id: &str,
    current: &mut String,
    options: &[(String, String)],
) -> egui::Response {
    ui.push_id(id, |ui| {
        let state = ui.make_persistent_id("model_choices_expanded");
        let mut expanded = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(state))
            .unwrap_or(false);
        let name = options
            .iter()
            .find(|(value, _)| value == current)
            .map_or_else(|| model_label(current), |(_, name)| name.clone());
        let response = ui
            .vertical_centered(|ui| {
                ui.button(
                    RichText::new(format!("{name} {}", if expanded { "−" } else { "+" }))
                        .size(12.0),
                )
            })
            .inner;
        if response.clicked() {
            expanded = !expanded;
        }
        if expanded {
            egui::ScrollArea::vertical()
                .id_salt("model_choices_scroll")
                .max_height(220.0)
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    for (value, label) in options {
                        if ui.selectable_value(current, value.clone(), label).clicked() {
                            expanded = false;
                        }
                    }
                    if options.is_empty() {
                        ui.label("Connect and refresh models to choose one.");
                    }
                });
        }
        ui.ctx().data_mut(|data| data.insert_temp(state, expanded));
        response
    })
    .inner
}

fn effort_steps(ui: &mut egui::Ui, labels: &[String], selected: &mut usize) -> egui::Response {
    let (rect, mut response) =
        ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click_and_drag());
    if labels.is_empty() {
        return response.on_hover_text("This model has no adjustable reasoning levels.");
    }
    let previous = *selected;
    *selected = (*selected).min(labels.len() - 1);
    let track = rect.shrink2(vec2(13.0, 3.0));
    if ui.is_enabled() {
        if (response.clicked() || response.dragged())
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let fraction = ((pointer.x - track.left()) / track.width()).clamp(0.0, 1.0);
            *selected = (fraction * (labels.len() - 1) as f32).round() as usize;
            response.request_focus();
        }
        if response.has_focus() {
            ui.input(|input| {
                if input.key_pressed(egui::Key::ArrowLeft) {
                    *selected = selected.saturating_sub(1);
                }
                if input.key_pressed(egui::Key::ArrowRight) {
                    *selected = (*selected + 1).min(labels.len() - 1);
                }
                if input.key_pressed(egui::Key::Home) {
                    *selected = 0;
                }
                if input.key_pressed(egui::Key::End) {
                    *selected = labels.len() - 1;
                }
            });
        }
    }
    if previous != *selected {
        response.mark_changed();
    }
    let fraction = if labels.len() > 1 {
        *selected as f32 / (labels.len() - 1) as f32
    } else {
        0.0
    };
    let x = egui::lerp(track.x_range(), fraction);
    let center = track.center().y;
    ui.painter().rect_filled(
        rect.shrink2(vec2(0.0, 3.0)),
        12,
        theme::muted().gamma_multiply(0.35),
    );
    let filled = egui::Rect::from_min_max(
        pos2(rect.left(), center - 12.0),
        pos2(x + 12.0, center + 12.0),
    );
    ui.painter().rect_filled(filled, 12, theme::blue());
    for index in 0..labels.len() {
        let fraction = if labels.len() > 1 {
            index as f32 / (labels.len() - 1) as f32
        } else {
            0.0
        };
        ui.painter().circle_filled(
            pos2(egui::lerp(track.x_range(), fraction), center),
            2.0,
            theme::muted(),
        );
    }
    ui.painter()
        .circle_filled(pos2(x, center), 13.0, egui::Color32::WHITE);
    if response.has_focus() {
        ui.painter()
            .circle_stroke(pos2(x, center), 14.0, egui::Stroke::new(1.0, theme::blue()));
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Slider,
            ui.is_enabled(),
            true,
            format!("Reasoning effort: {}", labels[*selected]),
        )
    });
    response.on_hover_text(format!(
        "{} · Click a step or use Left/Right. Reset restores the model default.",
        labels[*selected]
    ))
}

#[cfg(test)]
#[path = "../../tests/ui/effort_slider.rs"]
mod effort_slider_tests;

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
