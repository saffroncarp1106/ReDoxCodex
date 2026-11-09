use super::*;

const PALETTES: &[(&str, &[&str])] = &[
    (
        "Classic",
        &[
            "#F24E1E", "#A259FF", "#1ABCFE", "#0ACF83", "#FF7262", "#FFC700",
        ],
    ),
    (
        "Pastel",
        &[
            "#FFADAD", "#FFD6A5", "#FDFFB6", "#CAFFBF", "#A0C4FF", "#BDB2FF",
        ],
    ),
    (
        "Ocean",
        &["#00B4D8", "#48CAE4", "#90E0EF", "#0077B6", "#80FFDB"],
    ),
    (
        "Sunset",
        &["#FF595E", "#FF924C", "#FFCA3A", "#C77DFF", "#F28482"],
    ),
];

impl Startup {
    pub(in crate::app) fn settings(&mut self, ui: &mut egui::Ui) -> bool {
        ui.hyperlink_to(
            "Swarm Rebuild — Dhvanil Patel",
            "https://github.com/DhvanilPatel/swarm-cursors",
        );
        ui.hyperlink_to(
            "Original Swarm animation — Joel Califa",
            "https://califa.github.io/swarm-site/",
        );
        ui.label("Native Rust adaptation for ReDox.");
        ui.add_space(12.0);
        let config = &mut self.config;
        ui.checkbox(&mut config.enabled, "Play on startup");
        ui.checkbox(&mut config.show_names, "Show cursor names");
        ui.checkbox(&mut config.show_trails, "Show cursor trails");
        ui.checkbox(&mut config.show_guides, "Show drawing guides");
        ui.horizontal(|ui| {
            ui.label("Duration");
            ui.add(
                egui::DragValue::new(&mut config.duration_seconds)
                    .range(0.8..=20.0)
                    .speed(0.1)
                    .suffix(" s"),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Cursors");
            ui.add(egui::DragValue::new(&mut config.cursor_count).range(1..=usize::MAX));
        });
        ui.horizontal(|ui| {
            ui.label("Cursor size");
            ui.add(
                egui::DragValue::new(&mut config.cursor_size)
                    .range(12.0..=42.0)
                    .suffix(" px"),
            );
        });
        ui.add_space(12.0);
        ui.label(egui::RichText::new("Names").strong());
        ui.label("Edit or add names, then check which ones to use. Cursors cycle through your selection.");
        let mut remove = None;
        let selected_count = config
            .name_choices
            .iter()
            .filter(|selected| **selected)
            .count();
        crate::app::ui::scroll::ScrollArea::vertical()
            .id_salt("cursor_names")
            .max_height(180.0)
            .show(ui, |ui| {
                for index in 0..config.names.len() {
                    ui.push_id(("cursor_name", index), |ui| {
                        ui.horizontal(|ui| {
                            let selected = &mut config.name_choices[index];
                            ui.add_enabled(
                                selected_count > 1 || !*selected,
                                egui::Checkbox::without_text(selected),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut config.names[index])
                                    .char_limit(24)
                                    .desired_width(180.0),
                            );
                            if ui
                                .add_enabled(config.names.len() > 1, egui::Button::new("Remove"))
                                .clicked()
                            {
                                remove = Some(index);
                            }
                        });
                    });
                }
            });
        if let Some(index) = remove {
            config.names.remove(index);
            config.name_choices.remove(index);
            if !config.name_choices.iter().any(|selected| *selected) {
                config.name_choices[0] = true;
            }
        }
        if ui
            .add_enabled(config.names.len() < 32, egui::Button::new("Add name"))
            .clicked()
        {
            config.names.push("New name".into());
            config.name_choices.push(true);
        }
        ui.add_space(16.0);
        ui.label(egui::RichText::new("Color palette").strong());
        let palette = PALETTES
            .iter()
            .find(|(_, colors)| {
                config
                    .colors
                    .iter()
                    .map(String::as_str)
                    .eq(colors.iter().copied())
            })
            .map_or("Custom", |(name, _)| *name);
        egui::ComboBox::from_id_salt("cursor_palette")
            .selected_text(palette)
            .show_ui(ui, |ui| {
                for (name, colors) in PALETTES {
                    if ui.selectable_label(palette == *name, *name).clicked() {
                        config.colors = colors.iter().map(|color| (*color).to_owned()).collect();
                    }
                }
            });
        ui.label("Click a swatch to choose a custom color.");
        let mut remove = None;
        let color_count = config.colors.len();
        for (index, value) in config.colors.iter_mut().enumerate() {
            ui.push_id(("cursor_color", index), |ui| {
                ui.horizontal(|ui| {
                    let color = parse_color(value).unwrap_or(Color32::LIGHT_BLUE);
                    let mut rgb = [color.r(), color.g(), color.b()];
                    if ui.color_edit_button_srgb(&mut rgb).changed() {
                        *value = format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
                    }
                    ui.monospace(value.as_str());
                    if ui
                        .add_enabled(color_count > 1, egui::Button::new("Remove"))
                        .clicked()
                    {
                        remove = Some(index);
                    }
                });
            });
        }
        if let Some(index) = remove {
            config.colors.remove(index);
        }
        if ui
            .add_enabled(config.colors.len() < 32, egui::Button::new("Add color"))
            .clicked()
        {
            config.colors.push("#FFFFFF".into());
        }
        ui.add_space(16.0);
        let replay = ui.button("Replay animation").clicked();
        ui.label("Changes are saved automatically and used on the next startup or replay.");
        if ui.button("Restore animation defaults").clicked() {
            self.config = serde_json::from_str(include_str!("startup.json")).unwrap_or_default();
            self.config.normalize();
        }
        if replay {
            self.config.normalize();
            self.started = None;
            self.finished = false;
            ui.ctx().request_repaint();
        }
        replay
    }
}
