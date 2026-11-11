//! Stable host calls shared by Studio documents and ReDox.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiAction {
    #[serde(rename = "history")]
    History,
    #[serde(rename = "project")]
    Project,
    #[serde(rename = "project.open")]
    OpenProject,
    #[serde(rename = "chats")]
    Chats,
    #[serde(rename = "chat.new")]
    NewChat,
    #[serde(rename = "chat.send")]
    Send,
    #[serde(rename = "chat.stop")]
    Stop,
    #[serde(rename = "chat.previous")]
    Previous,
    #[serde(rename = "chat.next")]
    Next,
    #[serde(rename = "chat.rename")]
    Rename,
    #[serde(rename = "ollama.agent")]
    OllamaAgent,
    #[serde(rename = "settings")]
    Settings,
    #[serde(rename = "sidebar.toggle")]
    Sidebar,
    #[serde(rename = "files.add")]
    AddFiles,
    #[serde(rename = "menus")]
    Menus,
    #[serde(rename = "model")]
    Model,
    #[serde(rename = "effort")]
    Effort,
    #[serde(rename = "model.effort")]
    ModelEffort,
    #[serde(rename = "composer")]
    Composer,
    #[serde(rename = "conversation")]
    Conversation,
    #[serde(rename = "settings.account")]
    SettingsAccount,
    #[serde(rename = "settings.models")]
    SettingsModels,
    #[serde(rename = "settings.workspace")]
    SettingsWorkspace,
    #[serde(rename = "settings.appearance")]
    SettingsAppearance,
    #[serde(rename = "settings.cursors")]
    SettingsCursors,
    #[serde(rename = "settings.advanced")]
    SettingsAdvanced,
    #[serde(rename = "settings.updates")]
    SettingsUpdates,
    #[serde(rename = "settings.about")]
    SettingsAbout,
    #[serde(rename = "settings.close")]
    SettingsClose,
}

impl UiAction {
    pub const ALL: &'static [Self] = &[
        Self::History,
        Self::Project,
        Self::OpenProject,
        Self::Chats,
        Self::NewChat,
        Self::Send,
        Self::Stop,
        Self::Previous,
        Self::Next,
        Self::Rename,
        Self::OllamaAgent,
        Self::Settings,
        Self::Sidebar,
        Self::AddFiles,
        Self::Menus,
        Self::Model,
        Self::Effort,
        Self::ModelEffort,
        Self::Composer,
        Self::Conversation,
        Self::SettingsAccount,
        Self::SettingsModels,
        Self::SettingsWorkspace,
        Self::SettingsAppearance,
        Self::SettingsCursors,
        Self::SettingsAdvanced,
        Self::SettingsUpdates,
        Self::SettingsAbout,
        Self::SettingsClose,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::History => "History",
            Self::Project => "Project",
            Self::OpenProject => "Open project",
            Self::Chats => "My chats",
            Self::NewChat => "New chat",
            Self::Send => "Send",
            Self::Stop => "Stop response",
            Self::Previous => "Previous chat",
            Self::Next => "Next chat",
            Self::Rename => "Rename chat",
            Self::OllamaAgent => "Ollama agent",
            Self::Settings => "Settings",
            Self::Sidebar => "Toggle sidebar",
            Self::AddFiles => "Add files",
            Self::Menus => "Menu",
            Self::Model => "Model",
            Self::Effort => "Reasoning effort",
            Self::ModelEffort => "Model and effort",
            Self::Composer => "Message",
            Self::Conversation => "Conversation area",
            Self::SettingsAccount => "Account",
            Self::SettingsModels => "Models",
            Self::SettingsWorkspace => "Workspace",
            Self::SettingsAppearance => "Appearance",
            Self::SettingsCursors => "Cursors",
            Self::SettingsAdvanced => "Advanced",
            Self::SettingsUpdates => "Updates",
            Self::SettingsAbout => "About",
            Self::SettingsClose => "Close settings",
        }
    }
    pub fn id(self) -> String {
        serde_json::to_value(self).unwrap().as_str().unwrap().into()
    }
    pub fn is_control(self) -> bool {
        matches!(
            self,
            Self::Menus
                | Self::Model
                | Self::Effort
                | Self::ModelEffort
                | Self::Composer
                | Self::Conversation
        )
    }
    pub fn scene(self) -> Option<crate::scenes::SceneId> {
        use crate::scenes::SceneId;
        Some(match self {
            Self::Settings | Self::SettingsAccount => SceneId::Account,
            Self::SettingsModels => SceneId::Models,
            Self::SettingsWorkspace => SceneId::Workspace,
            Self::SettingsAppearance => SceneId::Appearance,
            Self::SettingsCursors => SceneId::Cursors,
            Self::SettingsAdvanced => SceneId::Advanced,
            Self::SettingsUpdates => SceneId::Updates,
            Self::SettingsAbout => SceneId::About,
            Self::SettingsClose => SceneId::Conversation,
            _ => return None,
        })
    }
    pub fn from_label(label: &str) -> Option<Self> {
        Some(match label.trim() {
            "Account" => Self::SettingsAccount,
            "Models & reasoning" | "Models" => Self::SettingsModels,
            "Workspace & access" | "Workspace" => Self::SettingsWorkspace,
            "Appearance" => Self::SettingsAppearance,
            "Cursor animation" | "Cursors" => Self::SettingsCursors,
            "Advanced" => Self::SettingsAdvanced,
            "Updates" => Self::SettingsUpdates,
            "About" => Self::SettingsAbout,
            "OK" | "Cancel" | "Close settings" => Self::SettingsClose,
            "Settings" | "Open settings" => Self::Settings,
            "Send" | "Send message" => Self::Send,
            "Stop" | "Stop response" => Self::Stop,
            "New chat" => Self::NewChat,
            "Previous chat" => Self::Previous,
            "Next chat" => Self::Next,
            "Toggle sidebar" => Self::Sidebar,
            "Open Project" | "Open project" | "Open project folder" => Self::OpenProject,
            "My Chats" | "Chats" | "History" => Self::History,
            "Ollama Agents" | "Agents" | "Ollama agent" | "Hide Agents" => Self::OllamaAgent,
            "+ Attach" | "Add files" => Self::AddFiles,
            _ => return None,
        })
    }
}
