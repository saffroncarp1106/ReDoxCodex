use eframe::egui;

use crate::app::ReDox;

pub(super) struct Renderer {
    pub id: &'static str,
    pub config: &'static str,
    pub rail: fn(&mut ReDox, &egui::Context),
    pub conversation: fn(&mut ReDox, &egui::Context),
}

pub(super) const RENDERERS: &[Renderer] = &[
    Renderer {
        id: "codex",
        config: include_str!("layouts/codex.json"),
        rail: ReDox::navigation_rail,
        conversation: ReDox::conversation,
    },
    Renderer {
        id: "gemini",
        config: include_str!("layouts/gemini.json"),
        rail: ReDox::gemini_rail,
        conversation: ReDox::gemini_conversation,
    },
    Renderer {
        id: "claude",
        config: include_str!("layouts/claude.json"),
        rail: ReDox::claude_rail,
        conversation: ReDox::claude_conversation,
    },
    Renderer {
        id: super::win98::ID,
        config: include_str!("layouts/win98.json"),
        rail: ReDox::win98_desktop,
        conversation: ReDox::win98_conversation,
    },
];

pub(super) fn resolve(id: &str) -> &'static Renderer {
    RENDERERS
        .iter()
        .find(|renderer| renderer.id == id)
        .unwrap_or(&RENDERERS[0])
}

#[cfg(test)]
#[path = "../../tests/ui/renderers.rs"]
mod tests;
