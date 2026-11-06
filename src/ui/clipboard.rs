use crate::app::ForgeDesk;
use eframe::egui;

#[derive(Default)]
pub(in crate::app) struct PasteGate {
    command_seen: bool,
    handled_press: bool,
}

impl PasteGate {
    pub fn requested(&mut self, input: &egui::RawInput) -> bool {
        if !input.focused {
            *self = Self::default();
            return false;
        }
        self.command_seen |= input.modifiers.command || input.modifiers.ctrl;
        let mut requested = false;
        for event in &input.events {
            match event {
                egui::Event::Paste(_) => {
                    requested = true;
                    self.handled_press = true;
                }
                egui::Event::Copy | egui::Event::Cut => self.command_seen = false,
                egui::Event::Key {
                    key: egui::Key::V,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.command || modifiers.ctrl => {
                    requested = true;
                    self.handled_press = true;
                }
                egui::Event::Key {
                    key: egui::Key::V,
                    pressed: false,
                    modifiers,
                    ..
                } => {
                    requested |= !self.handled_press
                        && (modifiers.command || modifiers.ctrl || self.command_seen);
                    self.handled_press = false;
                    self.command_seen = false;
                }
                egui::Event::Key {
                    pressed: true,
                    modifiers,
                    ..
                } if !modifiers.command && !modifiers.ctrl => self.command_seen = false,
                _ => {}
            }
        }
        requested
    }
}

impl ForgeDesk {
    pub(in crate::app) fn handle_clipboard_input(
        &mut self,
        context: &egui::Context,
        input: &mut egui::RawInput,
    ) {
        let requested = self.paste_gate.requested(input);
        if !requested
            || input.viewport_id != egui::ViewportId::ROOT
            || self.startup.is_active()
            || self.rename_open
            || (self.settings_open && !self.ui_preferences.floating_windows)
        {
            return;
        }
        let focused = context.memory(|memory| memory.focused());
        if focused.is_some() && focused != self.prompt_input_id {
            return;
        }
        if self.paste_clipboard_attachments(context) {
            input
                .events
                .retain(|event| !matches!(event, egui::Event::Paste(_)));
        }
    }
}

#[cfg(test)]
#[path = "../../tests/ui/clipboard.rs"]
mod tests;
