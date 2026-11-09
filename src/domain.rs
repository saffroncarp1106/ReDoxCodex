use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::attachments::Attachment;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const DEFAULT_MODEL: &str = "Account default";
pub const MAX_CONTEXT_BYTES: usize = 128 * 1024;

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Debug)]
pub enum Provider {
    #[default]
    Codex,
    Ollama,
}

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum Mode {
    Chat,
    #[default]
    Code,
    FullAccess,
}

impl Mode {
    pub fn sandbox(self) -> &'static str {
        match self {
            Self::Chat => "read-only",
            Self::Code => "workspace-write",
            Self::FullAccess => "danger-full-access",
        }
    }

    pub fn approval_policy(self) -> &'static str {
        if self == Self::FullAccess {
            "never"
        } else {
            "untrusted"
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Chat => "Read-only workspace",
            Self::Code => "Workspace access",
            Self::FullAccess => "Full access",
        }
    }
}

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Debug)]
pub enum MessageStatus {
    Queued,
    Streaming,
    Complete,
    Cancelled,
    Failed,
    Interrupted,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Message {
    pub id: Uuid,
    pub role: Role,
    pub content: String,
    pub status: MessageStatus,
    pub error: Option<String>,
    pub model: String,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(
        default,
        skip_serializing_if = "crate::activity::TurnActivity::is_empty"
    )]
    pub activity: crate::activity::TurnActivity,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RunId {
    pub id: Uuid,
    pub chat: Uuid,
    pub message: Uuid,
}

#[derive(Deserialize, Serialize)]
pub struct Chat {
    #[serde(default)]
    pub imported_history: bool,
    #[serde(skip)]
    pub send_after: BTreeMap<Uuid, Instant>,
    #[serde(default)]
    pub desktop_tools: bool,
    #[serde(default)]
    pub delegation_tools: bool,
    #[serde(default)]
    pub delegated_tasks: Vec<crate::delegation::Task>,
    #[serde(default)]
    pub provider: Provider,
    #[serde(default)]
    pub ollama_model: String,
    #[serde(default)]
    pub ollama_usage: crate::ollama::Usage,
    pub id: Uuid,
    pub title: String,
    pub draft: String,
    pub archived: bool,
    pub workspace: Option<PathBuf>,
    #[serde(default)]
    pub messages: Vec<Message>,
    #[serde(default)]
    pub thread_id: Option<String>,
    #[serde(default)]
    pub mode: Mode,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub previous_thread_ids: Vec<String>,
}

impl Chat {
    pub fn is_blank(&self) -> bool {
        !self.archived
            && self.title == "New chat"
            && self.messages.is_empty()
            && self.draft.trim().is_empty()
            && self.attachments.is_empty()
            && self.thread_id.is_none()
            && self.previous_thread_ids.is_empty()
            && self.delegated_tasks.is_empty()
    }

    pub fn new(workspace: Option<PathBuf>) -> Self {
        let mode = if workspace.is_some() {
            Mode::Code
        } else {
            Mode::Chat
        };
        Self {
            imported_history: false,
            send_after: BTreeMap::new(),
            desktop_tools: false,
            delegation_tools: false,
            delegated_tasks: Vec::new(),
            provider: Provider::Codex,
            ollama_model: String::new(),
            ollama_usage: crate::ollama::Usage::default(),
            id: Uuid::new_v4(),
            title: "New chat".into(),
            draft: String::new(),
            archived: false,
            workspace,
            messages: Vec::new(),
            thread_id: None,
            mode,
            attachments: Vec::new(),
            previous_thread_ids: Vec::new(),
        }
    }

    pub fn ready_queued(&self, now: Instant) -> Option<Uuid> {
        self.messages
            .iter()
            .find(|message| {
                message.status == MessageStatus::Queued
                    && self
                        .send_after
                        .get(&message.id)
                        .is_none_or(|due| *due <= now)
            })
            .map(|message| message.id)
    }

    pub fn cancel_queued(&mut self, id: Uuid, reason: String) {
        if let Some(message) = self
            .messages
            .iter_mut()
            .find(|message| message.id == id && message.status == MessageStatus::Queued)
        {
            self.send_after.remove(&id);
            message.status = MessageStatus::Interrupted;
            message.error = Some(reason);
        }
    }

    pub(crate) fn interrupt_queue(&mut self, reason: &str) {
        self.send_after.clear();
        for message in &mut self.messages {
            if message.status == MessageStatus::Queued {
                message.status = MessageStatus::Interrupted;
                message.error = Some(reason.into());
            }
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        let mode = if self.workspace.is_some() {
            mode
        } else {
            Mode::Chat
        };
        if self.mode != mode {
            self.reset_server_thread();
            self.mode = mode;
        }
    }

    pub fn set_workspace(&mut self, workspace: Option<PathBuf>) {
        if self.workspace != workspace {
            if workspace.is_none() {
                self.mode = Mode::Chat;
            } else if self.workspace.is_none() {
                self.mode = Mode::Code;
            }
            self.reset_server_thread();
            self.workspace = workspace;
        }
    }

    pub fn set_provider(&mut self, provider: Provider, model: String) {
        if self.provider != provider {
            self.reset_server_thread();
        }
        self.provider = provider;
        if provider == Provider::Ollama {
            self.ollama_model = model;
        }
    }

    fn reset_server_thread(&mut self) {
        self.desktop_tools = false;
        self.delegation_tools = false;
        if let Some(thread) = self.thread_id.take() {
            self.previous_thread_ids.push(thread);
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct Session {
    pub desktop: crate::desktop::Settings,
    pub archived_projects: Vec<PathBuf>,
    pub local_archive_overrides: BTreeMap<String, bool>,
    pub ollama: crate::ollama::Settings,
    pub chats: Vec<Chat>,
    pub active_chat: Option<Uuid>,
    pub workspace: Option<PathBuf>,
    pub codex_path: String,
    pub model: String,
    #[serde(default)]
    pub tool_access_version: u32,
    #[serde(default)]
    pub model_efforts: BTreeMap<String, String>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            desktop: crate::desktop::Settings::default(),
            archived_projects: Vec::new(),
            local_archive_overrides: BTreeMap::new(),
            ollama: crate::ollama::Settings::default(),
            chats: Vec::new(),
            active_chat: None,
            workspace: None,
            codex_path: crate::codex::find_executable(),
            model: DEFAULT_MODEL.into(),
            tool_access_version: 3,
            model_efforts: BTreeMap::new(),
        }
    }
}

impl Session {
    pub fn branch_project_chat(&mut self, workspace: &Path) -> Uuid {
        let belongs = |chat: &&Chat| !chat.archived && chat.workspace.as_deref() == Some(workspace);
        let source = self
            .active()
            .filter(belongs)
            .or_else(|| self.chats.iter().rev().find(belongs));
        let mut branch = Chat::new(Some(workspace.to_path_buf()));
        if let Some(source) = source {
            branch.provider = source.provider;
            branch.ollama_model = source.ollama_model.clone();
            branch.mode = source.mode;
            branch.messages = source
                .messages
                .iter()
                .filter(|m| m.status == MessageStatus::Complete)
                .cloned()
                .map(|mut message| {
                    message.id = Uuid::new_v4();
                    message
                })
                .collect();
            if !branch.messages.is_empty() {
                branch.title = format!("Branch · {}", source.title);
            }
        }
        let id = branch.id;
        self.chats.push(branch);
        id
    }

    pub fn queue_prompt(&mut self) -> Result<Uuid, String> {
        let chat = self.active_mut().ok_or("Create a chat first.")?;
        let content = chat.draft.trim().to_owned();
        if chat.archived || (content.is_empty() && chat.attachments.is_empty()) {
            return Err("Enter a prompt in an unarchived chat.".into());
        }
        let bytes = content.len()
            + chat
                .attachments
                .iter()
                .map(Attachment::text_bytes)
                .sum::<usize>()
            + chat
                .messages
                .iter()
                .filter(|m| m.status == MessageStatus::Queued)
                .map(|m| {
                    m.content.len()
                        + m.attachments
                            .iter()
                            .map(Attachment::text_bytes)
                            .sum::<usize>()
                })
                .sum::<usize>();
        if bytes > MAX_CONTEXT_BYTES {
            return Err(
                "Queued messages exceed the 128 KiB text budget. Your draft is kept.".into(),
            );
        }
        let id = Uuid::new_v4();
        chat.messages.push(Message {
            id,
            role: Role::User,
            content,
            status: MessageStatus::Queued,
            error: None,
            model: String::new(),
            attachments: std::mem::take(&mut chat.attachments),
            activity: Default::default(),
        });
        chat.draft.clear();
        Ok(id)
    }

    pub fn queue_prompt_after(&mut self, delay: Duration) -> Result<Uuid, String> {
        let deadline = Instant::now()
            .checked_add(delay)
            .ok_or("Delay is too long.")?;
        let id = self.queue_prompt()?;
        if !delay.is_zero()
            && let Some(chat) = self.active_mut()
        {
            chat.send_after.insert(id, deadline);
        }
        Ok(id)
    }

    pub fn stop_queued(&mut self, chat_id: Uuid) {
        if let Some(chat) = self.chats.iter_mut().find(|chat| chat.id == chat_id) {
            chat.interrupt_queue("Queued message was not sent. Copy it to send again.");
        }
    }

    pub fn recover(&mut self) {
        if self.model.contains("qwen")
            || self.model.contains("huihui")
            || self.model.contains(":latest")
        {
            self.model = DEFAULT_MODEL.into();
        }

        for chat in &mut self.chats {
            if let Some(thread) = &chat.thread_id {
                if chat.archived {
                    self.local_archive_overrides
                        .entry(thread.clone())
                        .or_insert(true);
                }
                if let Some(archived) = self.local_archive_overrides.get(thread) {
                    chat.archived = *archived;
                }
            }
            for task in &mut chat.delegated_tasks {
                if task.status == crate::delegation::Status::Running {
                    task.status = crate::delegation::Status::Interrupted;
                    task.progress.clear();
                }
            }
            if self.tool_access_version < 2 {
                chat.mode = Mode::Code;
                chat.reset_server_thread();
            } else if self.tool_access_version < 3 && chat.desktop_tools {
                chat.reset_server_thread();
            }
            if chat.workspace.is_none() {
                chat.set_mode(Mode::Chat);
            }
            for message in &mut chat.messages {
                if message.status == MessageStatus::Streaming {
                    message.status = MessageStatus::Interrupted;
                }
            }
            chat.interrupt_queue(
                "ReDox closed before this message was sent. Copy it to send again.",
            );
        }
        self.tool_access_version = 3;

        if self.active().is_none() {
            self.active_chat = self
                .chats
                .iter()
                .find(|chat| !chat.archived)
                .map(|chat| chat.id);
        }
    }

    pub fn prepare_run(&mut self) -> Result<RunId, String> {
        let queued = self
            .active()
            .and_then(|chat| chat.ready_queued(Instant::now()));
        self.prepare_selected_run(queued)
    }

    pub fn prepare_selected_run(&mut self, queued: Option<Uuid>) -> Result<RunId, String> {
        if self.active().is_some_and(|chat| chat.imported_history) {
            let source = self.active_mut().unwrap();
            let mut branch = Chat::new(source.workspace.clone());
            branch.title = format!("Branch ? {}", source.title);
            branch.messages = source.messages.clone();
            branch.mode = source.mode;
            branch.provider = source.provider;
            branch.draft = std::mem::take(&mut source.draft);
            branch.attachments = std::mem::take(&mut source.attachments);
            branch.send_after = std::mem::take(&mut source.send_after);
            self.active_chat = Some(branch.id);
            self.chats.push(branch);
        }
        let desktop = self.desktop.enabled && crate::desktop::SUPPORTED;
        if let Some(chat) = self.active_mut()
            && chat.provider == Provider::Codex
            && chat.desktop_tools != desktop
        {
            chat.reset_server_thread();
        }
        let delegate =
            self.ollama.codex_delegation && !self.ollama.preferred_model.trim().is_empty();
        if let Some(chat) = self.active_mut()
            && chat.provider == Provider::Codex
            && delegate
            && !chat.delegation_tools
        {
            chat.reset_server_thread();
        }
        if self.chats.iter().any(|chat| {
            chat.messages
                .iter()
                .any(|message| message.status == MessageStatus::Streaming)
        }) {
            return Err("Stop the current response before starting another.".into());
        }

        let model = if let Some(chat) = self
            .active()
            .filter(|chat| chat.provider == Provider::Ollama)
        {
            if chat.ollama_model.trim().is_empty() {
                return Err("Choose an Ollama model in Agents first.".into());
            }
            format!("Ollama · {}", chat.ollama_model)
        } else {
            self.model.trim().to_owned()
        };

        if model.is_empty() {
            return Err("Choose a model in settings first.".into());
        }

        let chat = self.active_mut().ok_or("Create a chat first.")?;
        let queued = queued
            .map(|id| {
                chat.messages
                    .iter()
                    .position(|m| m.id == id && m.status == MessageStatus::Queued)
                    .filter(|_| {
                        chat.send_after
                            .get(&id)
                            .is_none_or(|due| *due <= Instant::now())
                    })
                    .ok_or("That queued message is no longer ready to send.")
            })
            .transpose()?;
        let prompt = queued.map_or_else(
            || chat.draft.trim().to_owned(),
            |i| chat.messages[i].content.clone(),
        );

        if chat.archived || (queued.is_none() && prompt.is_empty() && chat.attachments.is_empty()) {
            return Err("Enter a prompt in an unarchived chat.".into());
        }

        let history_bytes: usize = if chat.provider == Provider::Codex && chat.thread_id.is_some() {
            0
        } else {
            chat.messages
                .iter()
                .filter(|message| message.status == MessageStatus::Complete)
                .map(|message| {
                    message.content.len()
                        + message
                            .attachments
                            .iter()
                            .map(Attachment::text_bytes)
                            .sum::<usize>()
                })
                .sum()
        };

        let attachments = queued.map_or(&chat.attachments, |i| &chat.messages[i].attachments);
        let attachment_bytes: usize = attachments.iter().map(Attachment::text_bytes).sum();
        if history_bytes
            .saturating_add(prompt.len())
            .saturating_add(attachment_bytes)
            > MAX_CONTEXT_BYTES
        {
            return Err(
                "This chat exceeds the 128 KiB text budget. Start a new chat; your draft is kept."
                    .into(),
            );
        }

        if chat.title == "New chat"
            && !chat
                .messages
                .iter()
                .any(|message| message.status == MessageStatus::Complete)
        {
            let title = if prompt.is_empty() {
                attachments[0].name()
            } else {
                prompt.clone()
            };
            chat.title = title
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(48)
                .collect();
        }

        if let Some(index) = queued {
            let mut message = chat.messages.remove(index);
            chat.send_after.remove(&message.id);
            message.status = MessageStatus::Complete;
            message.error = None;
            message.model = model.clone();
            chat.messages.push(message);
        } else {
            chat.messages.push(Message {
                id: Uuid::new_v4(),
                role: Role::User,
                content: prompt,
                status: MessageStatus::Complete,
                error: None,
                model: model.clone(),
                attachments: std::mem::take(&mut chat.attachments),
                activity: Default::default(),
            });
            chat.draft.clear();
        }

        let message_id = Uuid::new_v4();
        chat.messages.push(Message {
            id: message_id,
            role: Role::Assistant,
            content: String::new(),
            status: MessageStatus::Streaming,
            error: None,
            model,
            attachments: Vec::new(),
            activity: Default::default(),
        });

        Ok(RunId {
            id: Uuid::new_v4(),
            chat: chat.id,
            message: message_id,
        })
    }

    pub fn run_message_mut(&mut self, run: RunId) -> Option<&mut Message> {
        self.chats
            .iter_mut()
            .find(|chat| chat.id == run.chat)?
            .messages
            .iter_mut()
            .find(|message| message.id == run.message)
            .filter(|message| message.status == MessageStatus::Streaming)
    }

    pub fn finish_run(&mut self, run: RunId, status: MessageStatus, error: Option<String>) {
        if let Some(message) = self.run_message_mut(run) {
            message.status = status;
            message.error = error;
        }
        if status != MessageStatus::Complete
            && let Some(chat) = self.chats.iter_mut().find(|chat| chat.id == run.chat)
        {
            for task in &mut chat.delegated_tasks {
                if task.status == crate::delegation::Status::Running {
                    task.status = if status == MessageStatus::Cancelled {
                        crate::delegation::Status::Cancelled
                    } else {
                        crate::delegation::Status::Interrupted
                    };
                    task.progress.clear();
                }
            }
        }
    }

    pub fn new_chat(&mut self) {
        self.new_chat_in(None);
    }

    pub fn new_project_chat(&mut self, workspace: PathBuf) {
        self.new_chat_in(Some(workspace));
    }

    fn new_chat_in(&mut self, workspace: Option<PathBuf>) {
        let (provider, model) = self
            .active()
            .map_or((Provider::Codex, String::new()), |chat| {
                (chat.provider, chat.ollama_model.clone())
            });
        let matches = |chat: &&Chat| {
            chat.is_blank()
                && chat.workspace == workspace
                && chat.provider == provider
                && (provider != Provider::Ollama || chat.ollama_model == model)
        };
        let reusable = self
            .active()
            .filter(matches)
            .or_else(|| self.chats.iter().rev().find(matches))
            .map(|chat| chat.id);
        if let Some(id) = reusable {
            self.active_chat = Some(id);
            return;
        }
        let mut chat = Chat::new(workspace);
        if let Some(active) = self.active() {
            chat.provider = active.provider;
            chat.ollama_model = active.ollama_model.clone();
        }
        self.active_chat = Some(chat.id);
        self.chats.push(chat);
    }

    pub fn set_chat_archived(&mut self, id: Uuid, archived: bool) -> bool {
        let Some(target) = self.chats.iter().find(|chat| chat.id == id) else {
            return false;
        };
        let thread = target.thread_id.clone();
        let same = |chat: &Chat| {
            chat.id == id
                || thread
                    .as_ref()
                    .is_some_and(|thread| chat.thread_id.as_ref() == Some(thread))
        };
        if self.chats.iter().filter(|chat| same(chat)).any(|chat| {
            chat.messages
                .iter()
                .any(|message| message.status == MessageStatus::Streaming)
        }) {
            return false;
        }
        for chat in self.chats.iter_mut().filter(|chat| same(chat)) {
            chat.archived = archived;
            if archived {
                chat.interrupt_queue("Chat was archived before this message was sent.");
            }
        }
        if let Some(thread) = thread {
            self.local_archive_overrides.insert(thread, archived);
        }
        true
    }

    pub fn active(&self) -> Option<&Chat> {
        self.chats
            .iter()
            .find(|chat| Some(chat.id) == self.active_chat)
    }

    pub fn active_mut(&mut self) -> Option<&mut Chat> {
        self.chats
            .iter_mut()
            .find(|chat| Some(chat.id) == self.active_chat)
    }
}

#[cfg(test)]
#[path = "../tests/transport/chat_lifecycle.rs"]
mod lifecycle_tests;
