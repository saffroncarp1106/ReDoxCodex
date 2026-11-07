use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::app::ReDox;

mod agents;
mod capture;
mod claude;
pub(super) mod clipboard;
mod controls;
mod conversation;
pub(super) mod desktop_overlay;
mod dialogs;
mod gemini;
mod icons;
mod markdown;
mod media;
pub(super) mod packs;
mod picker;
pub(super) mod project_window;
mod renderers;
mod response_text;
mod settings;
mod shell;
pub(super) mod skins;
pub(super) mod startup;
pub(super) mod theme;
mod usage;
mod window_frame;

pub(super) const PREFERENCES_KEY: &str = "redox.ui.v1";

#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
pub(super) struct Preferences {
    pub project_glow: project_window::Glow,
    pub text_colors: std::collections::BTreeMap<String, theme::TextColors>,
    pub ui: String,
    pub sidebars: std::collections::BTreeMap<String, bool>,
    pub zoom: f32,
    pub sidebar_visible: bool,
    pub startup: Option<startup::Config>,
    pub agents_open: bool,
    pub agent_task: String,
    pub floating_windows: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            project_glow: project_window::Glow::default(),
            text_colors: std::collections::BTreeMap::new(),
            ui: "codex".into(),
            sidebars: std::collections::BTreeMap::new(),
            zoom: 1.0,
            sidebar_visible: true,
            startup: None,
            agents_open: false,
            agent_task: String::new(),
            floating_windows: true,
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
    Cursors,
    Advanced,
}

impl ReDox {
    pub(super) fn draw_ui(&mut self, context: &egui::Context) {
        let mut palette = self.active_ui.config.colors().unwrap_or_default();
        if let Some(colors) = self.ui_preferences.text_colors.get(&self.active_ui.id) {
            palette = colors.apply(palette);
        }
        theme::update_colors(context, palette);
        self.title_bar(context);
        self.agents_panel(context);
        let renderer = renderers::resolve(&self.active_ui.config.layout);
        (renderer.rail)(self, context);
        if self.sidebar_visible
            && (!self.ui_preferences.agents_open || context.viewport_rect().width() >= 1280.0)
        {
            self.sidebar(context);
        }
        (renderer.conversation)(self, context);
        self.rename_dialog(context);
        self.settings(context);
        self.approval_dialog(context);
        self.usage_dialog(context);
        self.window_interactions(context);
        self.startup.draw(context);
        self.ui_drop_hint(context);
        self.project_windows(context);
        self.capture_frame(context);
    }
}
