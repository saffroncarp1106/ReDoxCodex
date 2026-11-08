use std::path::PathBuf;

use eframe::egui::{self, Color32, RichText, vec2};

use super::{packs, renderers};
use crate::app::ReDox;

pub(in crate::app) struct Editor {
    config: packs::Config,
    image: Option<PathBuf>,
    texture: Option<egui::TextureHandle>,
    error: Option<String>,
}

impl Editor {
    fn from_pack(pack: &packs::Pack, context: &egui::Context) -> Self {
        let mut editor = Self {
            config: pack.config.clone(),
            image: None,
            texture: None,
            error: None,
        };
        editor.config.name = format!("My {} skin", pack.config.name);
        if let Some(asset) = &pack.config.background_image
            && let Ok(path) = packs::asset_path(&pack.path, asset)
        {
            editor.choose_image(path, context);
            editor.config.background_opacity = pack.config.background_opacity;
        }
        editor
    }

    fn choose_image(&mut self, path: PathBuf, context: &egui::Context) {
        match packs::load_background(&path) {
            Ok(image) => {
                if self.image.is_none() {
                    self.config.background_opacity = 0.25;
                }
                let rgba = image.to_rgba8();
                self.texture = Some(context.load_texture(
                    "skin_picture_preview",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [rgba.width() as usize, rgba.height() as usize],
                        rgba.as_raw(),
                    ),
                    egui::TextureOptions::LINEAR,
                ));
                self.image = Some(path);
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn preview(&self, ui: &mut egui::Ui) {
        let palette = self.config.colors().unwrap_or_default();
        let (rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), 160.0), egui::Sense::hover());
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 8, palette.background);
        if let Some(texture) = &self.texture {
            paint_background(&painter, rect, texture, self.config.background_opacity);
        }
        let sidebar = egui::Rect::from_min_size(rect.min, vec2(65.0, rect.height()));
        painter.rect_filled(sidebar, 0, palette.sidebar);
        painter.text(
            sidebar.center_top() + vec2(0.0, 22.0),
            egui::Align2::CENTER_CENTER,
            "Chats",
            egui::FontId::proportional(12.0),
            palette.text,
        );
        let center = rect.center() + vec2(32.0, -18.0);
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            &self.config.heading,
            egui::FontId::proportional(16.0),
            palette.text,
        );
        let input = egui::Rect::from_center_size(
            center + vec2(0.0, 55.0),
            vec2((rect.width() - 100.0).max(40.0), 42.0),
        );
        painter.rect_filled(input, 10, palette.input_background);
        painter.text(
            input.left_center() + vec2(10.0, 0.0),
            egui::Align2::LEFT_CENTER,
            &self.config.placeholder,
            egui::FontId::proportional(12.0),
            palette.muted,
        );
    }
}

pub(super) fn paint_background(
    painter: &egui::Painter,
    rect: egui::Rect,
    texture: &egui::TextureHandle,
    opacity: f32,
) {
    let image = texture.size_vec2();
    let scale = (rect.width() / image.x).max(rect.height() / image.y);
    let visible = rect.size() / (image * scale);
    let uv = egui::Rect::from_center_size(egui::pos2(0.5, 0.5), visible);
    painter.image(
        texture.id(),
        rect,
        uv,
        Color32::WHITE.gamma_multiply(opacity),
    );
}

impl ReDox {
    pub(in crate::app) fn skin_picture_drop(
        &mut self,
        paths: &[PathBuf],
        context: &egui::Context,
    ) -> bool {
        if self.skin_editor.is_none() || paths.len() != 1 {
            return false;
        }
        let path = &paths[0];
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp" | "gif") {
            return false;
        }
        if let Some(editor) = &mut self.skin_editor {
            editor.choose_image(path.clone(), context);
        }
        true
    }

    pub(in crate::app) fn begin_skin(&mut self, context: &egui::Context) {
        let mut editor = Editor::from_pack(&self.active_ui, context);
        if let Some(colors) = self.ui_preferences.text_colors.get(&self.active_ui.id) {
            for (target, value) in [
                (&mut editor.config.palette.text, colors.general),
                (&mut editor.config.palette.menu_text, colors.menu),
                (
                    &mut editor.config.palette.input_background,
                    colors.input_background,
                ),
            ] {
                if let Some([r, g, b]) = value {
                    *target = Some(format!("#{r:02X}{g:02X}{b:02X}"));
                }
            }
        }
        self.skin_editor = Some(editor);
        self.settings_page = super::SettingsPage::Appearance;
        self.settings_open = true;
    }

    pub(in crate::app) fn skin_settings(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("Create skin…").clicked() {
                self.begin_skin(context);
            }
            if ui.button("Share current skin…").clicked()
                && let Some(folder) = rfd::FileDialog::new()
                    .set_title("Choose a folder for the skin pack")
                    .pick_folder()
            {
                let image = self
                    .active_ui
                    .config
                    .background_image
                    .as_deref()
                    .map(|asset| packs::asset_path(&self.active_ui.path, asset))
                    .transpose();
                match image.and_then(|image| {
                    packs::save(&self.active_ui.config, image.as_deref(), &folder)
                }) {
                    Ok(id) => {
                        self.status = format!(
                            "Skin saved to {}. Share this folder.",
                            folder.join(id).display()
                        )
                    }
                    Err(error) => self.ui_error = Some(error),
                }
            }
        });
        if let Some(id) = self.skin_saved.clone() {
            ui.horizontal_wrapped(|ui| {
                ui.label("Your skin is saved and ready.");
                if ui
                    .add_enabled(
                        self.running.is_none() && !self.importing_ui,
                        egui::Button::new("Apply saved skin (restarts app)"),
                    )
                    .clicked()
                {
                    self.switch_ui(&id, context);
                }
            });
        }
        if self.status.starts_with("Skin saved to ") {
            ui.label(&self.status);
        }
        let Some(mut editor) = self.skin_editor.take() else {
            return;
        };
        ui.separator();
        ui.heading("Create your skin");
        ui.label("Start from your current skin. Preview changes here, then save a new copy.");
        editor.preview(ui);
        ui.label(RichText::new("Color and picture preview").small());
        ui.label("Skin name");
        ui.add(egui::TextEdit::singleline(&mut editor.config.name).char_limit(80));
        egui::ComboBox::from_id_salt("skin_layout")
            .selected_text(format!("Layout: {}", editor.config.layout))
            .show_ui(ui, |ui| {
                for renderer in renderers::RENDERERS {
                    let config: packs::Config =
                        serde_json::from_str(renderer.config).expect("Built-in skin");
                    ui.selectable_value(&mut editor.config.layout, renderer.id.into(), config.name);
                }
            });
        ui.label("Welcome heading");
        ui.add(egui::TextEdit::singleline(&mut editor.config.heading).char_limit(300));
        ui.label("Message placeholder");
        ui.add(egui::TextEdit::singleline(&mut editor.config.placeholder).char_limit(200));
        ui.checkbox(
            &mut editor.config.sidebar_default,
            "Show sidebar by default",
        );
        ui.collapsing("Colors", |ui| {
            let palette = editor.config.colors().unwrap_or_default();
            for (label, value, fallback) in [
                (
                    "Background",
                    &mut editor.config.palette.background,
                    palette.background,
                ),
                (
                    "Sidebar",
                    &mut editor.config.palette.sidebar,
                    palette.sidebar,
                ),
                (
                    "Navigation rail",
                    &mut editor.config.palette.rail,
                    palette.rail,
                ),
                (
                    "Panels",
                    &mut editor.config.palette.surface,
                    palette.surface,
                ),
                (
                    "Message box",
                    &mut editor.config.palette.input_background,
                    palette.input_background,
                ),
                ("Text", &mut editor.config.palette.text, palette.text),
                (
                    "Menu text",
                    &mut editor.config.palette.menu_text,
                    palette.menu_text,
                ),
                (
                    "Secondary text",
                    &mut editor.config.palette.muted,
                    palette.muted,
                ),
                ("Borders", &mut editor.config.palette.border, palette.border),
                ("Accent", &mut editor.config.palette.accent, palette.blue),
            ] {
                ui.horizontal(|ui| {
                    let color = value
                        .as_deref()
                        .and_then(|value| packs::color(value).ok())
                        .unwrap_or(fallback);
                    let mut rgb = [color.r(), color.g(), color.b()];
                    if ui.color_edit_button_srgb(&mut rgb).changed() {
                        *value = Some(format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]));
                    }
                    ui.label(label);
                });
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui.button("Choose background picture…").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Pictures", &["png", "jpg", "jpeg", "webp", "gif"])
                    .pick_file()
            {
                editor.choose_image(path, context);
            }
            if editor.image.is_some() && ui.button("Remove picture").clicked() {
                editor.image = None;
                editor.texture = None;
            }
        });
        if let Some(path) = &editor.image {
            ui.label(path.file_name().unwrap_or_default().to_string_lossy());
            super::theme::slider(
                ui,
                egui::Slider::new(&mut editor.config.background_opacity, 0.0..=1.0)
                    .text("Picture opacity"),
            );
        }
        ui.label(RichText::new("PNG, JPEG, WebP or GIF · up to 8 MiB and 4096 × 4096. GIF uses its first frame. Pictures fill the space without stretching.").small());

        if let Some(error) = &editor.error {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
        let mut keep = true;
        ui.horizontal_wrapped(|ui| {
            if ui.button("Save skin").clicked() {
                match packs::save(&editor.config, editor.image.as_deref(), &self.ui_folder) {
                    Ok(id) => {
                        self.scan_ui();
                        self.status = format!(
                            "Skin saved. Choose it from the UI menu: {}",
                            editor.config.name
                        );
                        self.ui_error = None;
                        self.skin_saved = Some(id);
                        keep = false;
                    }
                    Err(error) => editor.error = Some(error),
                }
            }
            if ui.button("Cancel").clicked() {
                keep = false;
            }
        });
        ui.label(RichText::new("Saving preserves the original. Choose your new skin from the UI menu to apply it; applying restarts the app.").small());
        if keep {
            self.skin_editor = Some(editor);
        }
        ui.separator();
    }
}

#[cfg(test)]
#[path = "../../tests/ui/skins.rs"]
mod tests;
