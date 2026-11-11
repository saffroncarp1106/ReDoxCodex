use crate::{
    actions::UiAction,
    document::Document,
    scenes::{self, SceneId},
};

#[derive(Default)]
pub struct Simulation {
    pub draft: String,
    pub messages: Vec<(String, String)>,
    pub model: String,
    pub effort: String,
    pub scene: SceneId,
    pub sidebar: bool,
    pub actual_size: bool,
    pub agents: bool,
    pub status: String,
    theme: String,
    last_chat_scene: SceneId,
    scenes: std::collections::HashMap<String, CachedScene>,
    edited: Option<Document>,
    pub model_selected: bool,
    pub effort_selected: bool,
    pub session: u64,
    pub focus_composer: bool,
}

struct CachedScene {
    document: Document,
    stamp: Option<(std::time::SystemTime, u64)>,
    checked: std::time::Instant,
    path: std::path::PathBuf,
}

impl CachedScene {
    fn load(path: std::path::PathBuf) -> Result<Self, String> {
        let document = Document::load(&path)?;
        Ok(Self {
            document,
            stamp: Self::stamp(&path),
            checked: std::time::Instant::now(),
            path,
        })
    }
    fn stamp(path: &std::path::Path) -> Option<(std::time::SystemTime, u64)> {
        let metadata = std::fs::metadata(path).ok()?;
        Some((metadata.modified().ok()?, metadata.len()))
    }
    fn refresh(&mut self) -> Option<String> {
        if self.checked.elapsed() < std::time::Duration::from_millis(500) {
            return None;
        }
        self.checked = std::time::Instant::now();
        let updated = Self::stamp(&self.path);
        if self.stamp == updated {
            return None;
        }
        match Document::load(&self.path) {
            Ok(document) => {
                self.document = document;
                self.stamp = updated;
                None
            }
            Err(error) => Some(format!("Scene reload paused: {error}")),
        }
    }
}

impl Simulation {
    pub fn reference_theme(&self) -> &str {
        &self.theme
    }

    pub fn set_reference_theme(&mut self, theme: &str) -> Result<bool, String> {
        if !scenes::THEMES.contains(&theme) {
            return Err("Choose a bundled reference theme".into());
        }
        if self.theme == theme {
            return Ok(false);
        }
        self.theme = theme.into();
        self.scenes.clear();
        Ok(true)
    }

    pub fn navigate(&mut self, scene: SceneId) {
        if matches!(scene, SceneId::Welcome | SceneId::Conversation) {
            self.last_chat_scene = scene;
        } else if matches!(self.scene, SceneId::Welcome | SceneId::Conversation) {
            self.last_chat_scene = self.scene;
        }
        self.scene = scene;
        self.agents = scene == SceneId::Agents;
        self.sidebar = scene == SceneId::History;
    }
    pub fn reset_for(&mut self, document: &Document) {
        self.theme = document
            .source
            .as_ref()
            .map(|s| s.theme.clone())
            .unwrap_or_else(|| "codex".into());
        self.scene = document
            .source
            .as_ref()
            .map(|s| s.scene)
            .unwrap_or_default();
        self.last_chat_scene = if self.scene == SceneId::Conversation {
            SceneId::Conversation
        } else {
            SceneId::Welcome
        };
        self.session = self.session.wrapping_add(1);
        self.focus_composer = false;
        self.model = "Studio demo".into();
        self.effort = "medium".into();
        self.draft.clear();
        self.messages.clear();
        self.scenes.clear();
        self.edited = Some(document.clone());
        self.model_selected = false;
        self.effort_selected = false;
        self.sidebar = self.scene == SceneId::History;
        self.agents = self.scene == SceneId::Agents;
        self.status = "Local playground ready".into();
    }
    pub fn dispatch(&mut self, action: UiAction) {
        if let Some(scene) = action.scene() {
            if action == UiAction::SettingsClose {
                self.scene = self.last_chat_scene;
            } else {
                self.navigate(scene);
            }
            return;
        }
        match action {
            UiAction::Send => {
                let prompt = self.draft.trim().to_owned();
                if !prompt.is_empty() {
                    self.messages.push((prompt, "Yatta yatta. Larp larp. This is a dummy response from the Studio playground.".into()));
                    self.draft.clear();
                    self.focus_composer = true;
                    self.scene = SceneId::Conversation;
                    self.last_chat_scene = SceneId::Conversation;
                }
            }
            UiAction::NewChat => {
                self.session = self.session.wrapping_add(1);
                self.focus_composer = true;
                self.messages.clear();
                self.draft.clear();
                self.scene = SceneId::Welcome;
                self.last_chat_scene = SceneId::Welcome;
            }
            UiAction::History | UiAction::Chats => {
                self.sidebar = true;
                self.navigate(SceneId::History);
                self.status = "Dummy chat history opened".into();
            }
            UiAction::Sidebar => {
                self.sidebar = !self.sidebar;
                self.navigate(if self.sidebar {
                    SceneId::History
                } else {
                    self.last_chat_scene
                });
            }
            UiAction::OllamaAgent => {
                self.agents = !self.agents;
                self.navigate(if self.agents {
                    SceneId::Agents
                } else {
                    self.last_chat_scene
                });
                self.status = "Simulated Ollama agent ready".into();
            }
            UiAction::OpenProject | UiAction::Project => {
                self.status = "Opened dummy project: Studio playground".into()
            }
            UiAction::AddFiles => self.status = "Attached dummy-example.txt".into(),
            UiAction::Stop => self.status = "Dummy response stopped".into(),
            UiAction::Rename => self.status = "Dummy chat renamed: Skin experiment".into(),
            UiAction::Previous | UiAction::Next => {
                self.navigate(if self.scene == SceneId::Welcome {
                    SceneId::Conversation
                } else {
                    SceneId::Welcome
                });
            }
            _ => {}
        }
    }
    pub fn sync_editor(&mut self, editing: &Document) -> bool {
        if self.theme.is_empty() {
            self.reset_for(editing);
            return true;
        } else if self.edited.as_ref() != Some(editing) {
            if let Some(theme) = editing.source.as_ref().map(|s| s.theme.as_str())
                && self.theme != theme
            {
                self.theme = theme.into();
                self.scenes.clear();
            }
            self.return_to_editor(editing);
            self.edited = Some(editing.clone());
            return true;
        }
        false
    }

    pub fn return_to_editor(&mut self, editing: &Document) {
        self.scene = editing.source.as_ref().map(|s| s.scene).unwrap_or_default();
    }

    pub fn is_editing_scene(&self, editing: &Document) -> bool {
        self.scene == editing.source.as_ref().map(|s| s.scene).unwrap_or_default()
            || (editing.source.is_none()
                && matches!(self.scene, SceneId::Welcome | SceneId::Conversation))
    }

    pub fn document(&mut self, editing: &Document) -> Result<Document, String> {
        self.sync_editor(editing);
        let mut doc = if self.is_editing_scene(editing) {
            editing.clone()
        } else {
            let key = self.scene.id().to_owned();
            if let Some(cached) = self.scenes.get_mut(&key) {
                let previous = cached.stamp;
                if let Some(error) = cached.refresh() {
                    self.status = error;
                } else if previous != cached.stamp {
                    self.status = "Source scene reloaded".into();
                }
            } else {
                let path = scenes::path(&self.theme, self.scene)?;
                self.scenes.insert(key.clone(), CachedScene::load(path)?);
            }
            self.scenes[&key].document.clone()
        };
        if self.scene == SceneId::Conversation
            && let Some((prompt, answer)) = self.messages.last()
        {
            for node in doc.pages.iter_mut().flat_map(|p| &mut p.nodes) {
                // Authored labels win over simulated fixture text.
                let unchanged = node
                    .artwork
                    .as_ref()
                    .is_some_and(|art| node.text == art.original_text);
                if unchanged && node.text.starts_with("Dummy text.") {
                    node.text = prompt.clone();
                }
                if unchanged && node.text.starts_with("Yatta yatta.") {
                    node.text = answer.clone();
                }
            }
        }
        Ok(doc)
    }
}

#[cfg(test)]
#[path = "../tests/simulation/scene_reload.rs"]
mod scene_reload;
