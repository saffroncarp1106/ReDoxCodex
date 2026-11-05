use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::app::ForgeDesk;

mod capture;
mod controls;
mod conversation;
mod dialogs;
mod gemini;
mod icons;
mod markdown;
pub(super) mod packs;
mod picker;
mod settings;
mod shell;
pub(super) mod startup;
pub(super) mod theme;
mod usage;
mod window_frame;

pub(super) const PREFERENCES_KEY: &str = "codex-rust.ui.v1";

#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
pub(super) struct Preferences {
    pub ui: String,
    pub sidebars: std::collections::BTreeMap<String, bool>,
    pub zoom: f32,
    pub sidebar_visible: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            ui: "codex".into(),
            sidebars: std::collections::BTreeMap::new(),
            zoom: 1.0,
            sidebar_visible: true,
        }
    }
}

impl Preferences {
    pub fn normalize(&mut self) {
        self.zoom = if self.zoom.is_finite() {
            self.zoom.clamp(0.8, 1.5)
        } else {
            1.0
        };
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum SettingsPage {
    #[default]
    Account,
    Models,
    Workspace,
    Appearance,
    Advanced,
}

impl ForgeDesk {
    pub(super) fn draw_ui(&mut self, context: &egui::Context) {
        self.title_bar(context);
        if self.active_ui.config.layout == packs::Layout::Gemini {
            self.gemini_rail(context);
        } else {
            self.navigation_rail(context);
        }
        if self.sidebar_visible {
            self.sidebar(context);
        }
        if self.active_ui.config.layout == packs::Layout::Gemini {
            self.gemini_conversation(context);
        } else {
            self.conversation(context);
        }
        self.rename_dialog(context);
        self.settings(context);
        self.approval_dialog(context);
        self.usage_dialog(context);
        self.window_interactions(context);
        self.startup.draw(context);
        self.ui_drop_hint(context);
        self.capture_frame(context);
    }
}
