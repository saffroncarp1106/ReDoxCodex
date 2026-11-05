use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::attachments::Attachment;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const DEFAULT_MODEL: &str = "Account default";
pub const MAX_CONTEXT_BYTES: usize = 128 * 1024;

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
    Streaming,
    Complete,
    Cancelled,
    Failed,
    Interrupted,
}

#[derive(Deserialize, Serialize)]
pub struct Message {
    pub id: Uuid,
    pub role: Role,
    pub content: String,
    pub status: MessageStatus,
    pub error: Option<String>,
    pub model: String,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RunId {
    pub id: Uuid,
    pub chat: Uuid,
    pub message: Uuid,
}

#[derive(Deserialize, Serialize)]
pub struct Chat {
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
    pub fn new(workspace: Option<PathBuf>) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: "New chat".into(),
            draft: String::new(),
            archived: false,
            workspace,
            messages: Vec::new(),
            thread_id: None,
            mode: Mode::default(),
            attachments: Vec::new(),
            previous_thread_ids: Vec::new(),
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        if self.mode != mode {
            self.reset_server_thread();
            self.mode = mode;
        }
    }

    fn reset_server_thread(&mut self) {
        if let Some(thread) = self.thread_id.take() {
            self.previous_thread_ids.push(thread);
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct Session {
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
            chats: Vec::new(),
            active_chat: None,
            workspace: None,
            codex_path: crate::codex::find_executable(),
            model: DEFAULT_MODEL.into(),
            tool_access_version: 2,
            model_efforts: BTreeMap::new(),
        }
    }
}

impl Session {
    pub fn recover(&mut self) {
        if self.model.contains("qwen")
            || self.model.contains("huihui")
            || self.model.contains(":latest")
        {
            self.model = DEFAULT_MODEL.into();
        }

        for chat in &mut self.chats {
            if self.tool_access_version < 2 {
                chat.mode = Mode::Code;
                chat.reset_server_thread();
            }
            for message in &mut chat.messages {
                if message.status == MessageStatus::Streaming {
                    message.status = MessageStatus::Interrupted;
                }
            }
        }
        self.tool_access_version = 2;

        if self.active().is_none() {
            self.active_chat = self
                .chats
                .iter()
                .find(|chat| !chat.archived)
                .map(|chat| chat.id);
        }
    }

    pub fn prepare_run(&mut self) -> Result<RunId, String> {
        if self.chats.iter().any(|chat| {
            chat.messages
                .iter()
                .any(|message| message.status == MessageStatus::Streaming)
        }) {
            return Err("Stop the current response before starting another.".into());
        }

        let model = self.model.trim().to_owned();

        if model.is_empty() {
            return Err("Choose a model in settings first.".into());
        }

        let workspace = self.workspace.clone();
        let chat = self.active_mut().ok_or("Create a chat first.")?;
        if chat.workspace.is_none() {
            chat.workspace = workspace;
        }
        let prompt = chat.draft.trim().to_owned();

        if chat.archived || (prompt.is_empty() && chat.attachments.is_empty()) {
            return Err("Enter a prompt in an unarchived chat.".into());
        }

        let history_bytes: usize = if chat.thread_id.is_some() {
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

        let attachment_bytes: usize = chat.attachments.iter().map(Attachment::text_bytes).sum();
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

        if chat.messages.is_empty() && chat.title == "New chat" {
            let title = if prompt.is_empty() {
                chat.attachments[0].name()
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

        chat.messages.push(Message {
            id: Uuid::new_v4(),
            role: Role::User,
            content: prompt,
            status: MessageStatus::Complete,
            error: None,
            model: model.clone(),
            attachments: std::mem::take(&mut chat.attachments),
        });
        chat.draft.clear();

        let message_id = Uuid::new_v4();
        chat.messages.push(Message {
            id: message_id,
            role: Role::Assistant,
            content: String::new(),
            status: MessageStatus::Streaming,
            error: None,
            model,
            attachments: Vec::new(),
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
    }

    pub fn new_chat(&mut self) {
        let chat = Chat::new(self.workspace.clone());
        self.active_chat = Some(chat.id);
        self.chats.push(chat);
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
