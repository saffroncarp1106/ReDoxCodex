use std::path::Path;

use eframe::egui::{self, Color32, Rect, Stroke};
use serde::{Deserialize, Serialize};

use crate::app::ReDox;

#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
pub(in crate::app) struct Glow {
    enabled: bool,
    color: [u8; 3],
    strength: f32,
    radius: f32,
    speed: f32,
}

impl Default for Glow {
    fn default() -> Self {
        Self {
            enabled: true,
            color: [140, 105, 184],
            strength: 0.5,
            radius: 7.0,
            speed: 1.2,
        }
    }
}

impl Glow {
    pub fn settings(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Project folder border glow").strong());
        ui.checkbox(&mut self.enabled, "Show glow when hovering the border");
        ui.horizontal(|ui| {
            ui.color_edit_button_srgb(&mut self.color);
            ui.label("Glow color");
        });
        ui.add(egui::Slider::new(&mut self.strength, 0.1..=1.0).text("Intensity"));
        ui.add(egui::Slider::new(&mut self.radius, 2.0..=14.0).text("Aura size"));
        ui.add(egui::Slider::new(&mut self.speed, 0.2..=3.0).text("Pulse speed"));
        ui.label("Drag a project folder row out to open a chat branch in a separate window.");
    }
}

pub(in crate::app) struct ProjectWindow {
    title: String,
    chat: uuid::Uuid,
    position: egui::Pos2,
    focus_prompt: bool,
}

fn border_contains(rect: Rect, point: egui::Pos2) -> bool {
    rect.contains(point) && !rect.shrink(4.0).contains(point)
}

fn detach_position(response: &egui::Response) -> Option<egui::Pos2> {
    if !response.drag_stopped_by(egui::PointerButton::Primary) {
        return None;
    }
    response
        .interact_pointer_pos()
        .or_else(|| response.ctx.input(|i| i.pointer.latest_pos()))
        .filter(|p| !response.rect.expand(18.0).contains(*p))
}

impl ReDox {
    pub(in crate::app) fn project_drag(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        path: &Path,
    ) {
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let border =
            response.hovered() && pointer.is_some_and(|p| border_contains(response.rect, p));
        if border {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            let glow = &self.ui_preferences.project_glow;
            if glow.enabled {
                let time = ui.input(|i| i.time) as f32;
                let pulse =
                    0.7 + 0.3 * (time * glow.speed.clamp(0.2, 3.0) * std::f32::consts::TAU).sin();
                let radius = glow.radius.clamp(2.0, 14.0);
                let color = Color32::from_rgb(glow.color[0], glow.color[1], glow.color[2]);
                for layer in (0..8).rev() {
                    let spread = layer as f32 * radius / 8.0;
                    let opacity =
                        (1.0 - layer as f32 / 8.0) * glow.strength.clamp(0.1, 1.0) * pulse;
                    ui.painter().rect_stroke(
                        response.rect.expand(spread),
                        10.0 + spread,
                        Stroke::new(1.5, color.gamma_multiply(opacity)),
                        egui::StrokeKind::Outside,
                    );
                }
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(33));
            }
        }
        if let Some(position) = detach_position(response) {
            let position = ui.input(|i| i.viewport().inner_rect.map(|r| r.min).unwrap_or_default())
                + position.to_vec2();
            let name = path
                .file_name()
                .unwrap_or(path.as_os_str())
                .to_string_lossy();
            self.project_windows.push(ProjectWindow {
                title: format!("{name} · Chat branch"),
                chat: self.session.branch_project_chat(path),
                position,
                focus_prompt: true,
            });
        }
    }

    pub(in crate::app) fn project_windows(&mut self, context: &egui::Context) {
        let windows = std::mem::take(&mut self.project_windows);
        for mut window in windows {
            let id = egui::ViewportId::from_hash_of(("project_window", window.chat));
            let mut open = true;
            context.show_viewport_immediate(
                id,
                Self::app_viewport()
                    .with_title(window.title.clone())
                    .with_position(window.position)
                    .with_inner_size([800.0, 640.0])
                    .with_min_inner_size([540.0, 360.0])
                    .with_decorations(true),
                |child, class| {
                    if child.input(|i| i.viewport().close_requested()) {
                        open = false;
                        child.request_repaint_of(egui::ViewportId::ROOT);
                        return;
                    }
                    if class == egui::ViewportClass::Embedded {
                        egui::Window::new(&window.title)
                            .open(&mut open)
                            .show(child, |ui| {
                                ui.label("Open this branch from the main chat list.");
                            });
                    } else {
                        let selected = self.session.active_chat.replace(window.chat);
                        let prompt_input_id = self.prompt_input_id.take();
                        std::mem::swap(&mut self.focus_prompt, &mut window.focus_prompt);
                        let renderer = super::renderers::resolve(&self.active_ui.config.layout);
                        (renderer.conversation)(self, child);
                        std::mem::swap(&mut self.focus_prompt, &mut window.focus_prompt);
                        self.prompt_input_id = prompt_input_id;
                        self.session.active_chat = selected;
                    }
                    if child.input(|i| !i.events.is_empty()) {
                        child.request_repaint_of(egui::ViewportId::ROOT);
                    }
                },
            );
            if open {
                self.project_windows.push(window);
            }
        }
    }
}

#[cfg(test)]
#[path = "../../tests/ui/project_window.rs"]
mod tests;
