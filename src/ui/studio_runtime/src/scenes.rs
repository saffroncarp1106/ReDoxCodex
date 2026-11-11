use crate::document::Document;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SceneId {
    #[default]
    Welcome,
    Conversation,
    Agents,
    History,
    Account,
    Models,
    Workspace,
    Appearance,
    Cursors,
    Advanced,
    Updates,
    About,
}

impl SceneId {
    pub const ALL: [Self; 12] = [
        Self::Welcome,
        Self::Conversation,
        Self::Agents,
        Self::History,
        Self::Account,
        Self::Models,
        Self::Workspace,
        Self::Appearance,
        Self::Cursors,
        Self::Advanced,
        Self::Updates,
        Self::About,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Self::Welcome => "welcome",
            Self::Conversation => "conversation",
            Self::Agents => "agents",
            Self::History => "history",
            Self::Account => "account",
            Self::Models => "models",
            Self::Workspace => "workspace",
            Self::Appearance => "appearance",
            Self::Cursors => "cursors",
            Self::Advanced => "advanced",
            Self::Updates => "updates",
            Self::About => "about",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Welcome => "Welcome",
            Self::Conversation => "Conversation",
            Self::Agents => "Agents",
            Self::History => "My chats",
            Self::Account => "Account",
            Self::Models => "Models",
            Self::Workspace => "Workspace",
            Self::Appearance => "Appearance",
            Self::Cursors => "Cursors",
            Self::Advanced => "Advanced",
            Self::Updates => "Updates",
            Self::About => "About",
        }
    }
}

pub const THEMES: [&str; 5] = ["codex", "claude", "gemini", "win98", "wii"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub theme: String,
    pub scene: SceneId,
}

pub fn path(theme: &str, scene: SceneId) -> Result<std::path::PathBuf, String> {
    if !THEMES.contains(&theme) {
        return Err("Unknown source theme".into());
    }
    let filename = format!("{}-{}.redoxskin", theme, scene.id());
    let mut roots =
        vec![std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("skins/redox/scenes")];
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        roots.insert(0, parent.join("skins/redox/scenes"));
    }
    for root in roots {
        let path = root.join(&filename);
        if path.is_file() {
            return Ok(path);
        }
    }
    Err(format!(
        "Source scene {filename} is missing. Run Redox's export-designs command and package skins/redox/scenes with Studio."
    ))
}

pub fn load(theme: &str, scene: SceneId) -> Result<Document, String> {
    Document::load(&path(theme, scene)?)
}
