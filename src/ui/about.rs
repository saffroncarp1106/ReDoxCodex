use eframe::egui::{self, Align2, Color32, Frame, Margin, Sense, Stroke, vec2};

use super::{Preferences, theme, win98};
use crate::app::ReDox;

#[derive(Default)]
pub(in crate::app) struct About {
    logo_clicks: u8,
    pub(super) show_notice: bool,
}

impl About {
    fn click_logo(&mut self, preferences: &mut Preferences) {
        if preferences.win98_unlocked {
            return;
        }
        self.logo_clicks += 1;
        if self.logo_clicks == 5 {
            preferences.win98_unlocked = true;
            self.show_notice = true;
        }
    }
}

impl ReDox {
    pub(super) fn about_settings(&mut self, ui: &mut egui::Ui) {
        let context = ui.ctx().clone();
        let texture_id = egui::Id::new("about_redox_logo");
        let cached = context.data(|data| data.get_temp::<egui::TextureHandle>(texture_id));
        let logo = cached.unwrap_or_else(|| {
            let icon = Self::app_icon();
            let texture = context.load_texture(
                "ReDox logo",
                egui::ColorImage::from_rgba_unmultiplied(
                    [icon.width as usize, icon.height as usize],
                    &icon.rgba,
                ),
                egui::TextureOptions::LINEAR,
            );
            context.data_mut(|data| data.insert_temp(texture_id, texture.clone()));
            texture
        });
        ui.horizontal(|ui| {
            let logo = egui::Image::new(&logo).fit_to_exact_size(vec2(48.0, 48.0));
            if ui
                .add(egui::Button::image(logo).frame(false))
                .on_hover_text("ReDox logo")
                .clicked()
            {
                self.about.click_logo(&mut self.ui_preferences);
            }
            ui.vertical(|ui| {
                ui.heading("ReDox");
                ui.label(concat!("Version ", env!("CARGO_PKG_VERSION")));
            });
        });
        ui.label("A native Rust client for Codex and Ollama.");
        if self.about.show_notice {
            self.about.show_notice = !unlock_notice(&context);
        }
    }
}

fn unlock_notice(context: &egui::Context) -> bool {
    let silver = Color32::from_gray(192);
    let ink = Color32::BLACK;
    let frame = Frame::NONE
        .fill(silver)
        .stroke(Stroke::new(1.0, ink))
        .inner_margin(3);
    let modal = egui::Modal::new(egui::Id::new("win98_unlock_notice"))
        .frame(frame)
        .backdrop_color(Color32::from_black_alpha(46))
        .show(context, |ui| {
            let width = (context.content_rect().width() - 24.0).clamp(240.0, 410.0);
            ui.set_width(width);
            ui.spacing_mut().item_spacing = vec2(5.0, 5.0);
            ui.spacing_mut().button_padding = vec2(4.0, 1.0);
            ui.spacing_mut().interact_size.y = 19.0;
            ui.visuals_mut().override_text_color = Some(ink);
            let (caption, _) = ui.allocate_exact_size(vec2(width, 24.0), Sense::hover());
            ui.painter().rect_filled(caption, 0, win98::NAVY);
            ui.painter().text(
                caption.left_center() + vec2(6.0, 0.0),
                Align2::LEFT_CENTER,
                "New UI",
                theme::semibold(14.0),
                Color32::WHITE,
            );
            let close_rect = egui::Rect::from_center_size(
                caption.right_center() - vec2(12.0, 0.0),
                vec2(19.0, 19.0),
            );
            let close = ui
                .put(
                    close_rect,
                    egui::Button::new("×").fill(silver).corner_radius(0),
                )
                .on_hover_text("Close");
            win98::bevel(ui.painter(), close.rect, close.is_pointer_button_down_on());
            let body = Frame::NONE
                .inner_margin(Margin::symmetric(14, 14))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::hover());
                        let center = rect.center();
                        let painter = ui.painter();
                        painter.circle_filled(
                            center + vec2(1.0, 1.0),
                            17.0,
                            Color32::from_rgb(107, 23, 20),
                        );
                        painter.circle_filled(center, 16.0, Color32::from_rgb(192, 38, 30));
                        for direction in [vec2(7.0, 7.0), vec2(7.0, -7.0)] {
                            painter.line_segment(
                                [center - direction, center + direction],
                                Stroke::new(4.0, Color32::WHITE),
                            );
                        }
                        ui.add_space(6.0);
                        ui.add(egui::Label::new(
                            "Windows 98 unlocked. Choose it from the UI menu whenever you want.",
                        ).wrap());
                    });
                    ui.add_space(12.0);
                    ui.vertical_centered(|ui| {
                        let ok = ui.add_sized(
                            [86.0, 26.0],
                            egui::Button::new("OK").fill(silver).corner_radius(0),
                        );
                        win98::bevel(ui.painter(), ok.rect, ok.is_pointer_button_down_on());
                        ok.clicked()
                    })
                    .inner
                });
            close.clicked() || body.inner
        });
    modal.inner
        || (modal.is_top_modal
            && !modal.any_popup_open
            && context
                .input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)))
}

#[cfg(test)]
#[path = "../../tests/ui/about.rs"]
mod tests;
