use std::path::PathBuf;

use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::codex::Rpc;
use crate::domain::{Chat, Message, MessageStatus, Role, Session};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub name: Option<String>,
    #[serde(default)]
    pub preview: String,
    pub cwd: PathBuf,
    pub model: Option<String>,
}

impl HistoryEntry {
    pub fn title(&self) -> String {
        self.name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(&self.preview)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(100)
            .collect::<String>()
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub data: Vec<HistoryEntry>,
    pub next_cursor: Option<String>,
}

pub struct ConversationPage {
    pub entry: HistoryEntry,
    pub messages: Vec<Message>,
    pub next_cursor: Option<String>,
    pub archived: bool,
}

pub async fn list(
    executable: &str,
    cursor: Option<String>,
    archived: bool,
    search: &str,
) -> Result<HistoryPage, String> {
    let mut rpc = Rpc::connect(executable).await?;
    let result = rpc
        .call(
            "thread/list",
            json!({
                "limit":50, "cursor":cursor, "archived":archived,
                "sortKey":"updated_at", "sortDirection":"desc", "modelProviders":[],
                "searchTerm":if search.trim().is_empty() { None } else { Some(search.trim()) },
            }),
        )
        .await;
    rpc.close().await;
    serde_json::from_value(result?)
        .map_err(|error| format!("Could not read Codex history: {error}"))
}

pub async fn read(
    executable: &str,
    thread: &str,
    cursor: Option<String>,
    archived: bool,
) -> Result<ConversationPage, String> {
    let mut rpc = Rpc::connect(executable).await?;
    let result = async {
        let metadata = rpc
            .call(
                "thread/read",
                json!({"threadId":thread,"includeTurns":false}),
            )
            .await?;
        let entry: HistoryEntry = serde_json::from_value(metadata["thread"].clone())
            .map_err(|error| format!("Could not read conversation details: {error}"))?;
        let turns = rpc.call("thread/turns/list", json!({
            "threadId":thread,"cursor":cursor,"limit":20,"itemsView":"full","sortDirection":"desc",
        })).await?;
        let messages = parse_turns(&turns, entry.model.as_deref().unwrap_or("Codex"))?;
        Ok(ConversationPage {
            entry,
            messages,
            next_cursor: turns["nextCursor"].as_str().map(str::to_owned),
            archived,
        })
    }
    .await;
    rpc.close().await;
    result
}

pub fn parse_turns(page: &Value, model: &str) -> Result<Vec<Message>, String> {
    let turns = page["data"]
        .as_array()
        .ok_or("Codex did not return conversation turns.")?;
    let mut messages = Vec::new();
    for turn in turns.iter().rev() {
        let items = turn["items"]
            .as_array()
            .ok_or("Codex returned a turn without items.")?;
        let turn_id = turn["id"].as_str().unwrap_or_default();
        let mut activity = crate::activity::TurnActivity::default();
        let mut last_reply = None;
        for item in items {
            if let Some(command) = crate::activity::CommandActivity::from_item(item) {
                activity.record_command(command);
                continue;
            }
            let (role, content) = match item["type"].as_str() {
                Some("userMessage") => (
                    Role::User,
                    item["content"]
                        .as_array()
                        .map(|parts| {
                            parts
                                .iter()
                                .filter_map(|part| match part["type"].as_str() {
                                    Some("text") => part["text"].as_str().map(str::to_owned),
                                    Some("localImage") => {
                                        part["path"].as_str().map(crate::media::image_markdown)
                                    }
                                    Some("image") => part["url"]
                                        .as_str()
                                        .map(crate::media::image_markdown)
                                        .or_else(|| Some("[Image attachment unavailable]".into())),
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join("\n")
                        })
                        .unwrap_or_default(),
                ),
                Some("agentMessage") => (
                    Role::Assistant,
                    item["text"].as_str().unwrap_or_default().to_owned(),
                ),
                Some("imageGeneration") => (
                    Role::Assistant,
                    crate::media::generated_image(item).unwrap_or_default(),
                ),
                _ => continue,
            };
            if content.is_empty() {
                continue;
            }
            let failed = role == Role::Assistant && turn["status"] == "failed";
            let activity = if role == Role::Assistant {
                last_reply = Some(messages.len());
                std::mem::take(&mut activity)
            } else {
                Default::default()
            };
            messages.push(Message {
                id: Uuid::new_v4(),
                role,
                content,
                status: if failed {
                    MessageStatus::Failed
                } else if role == Role::Assistant
                    && (turn["status"] == "interrupted" || turn["status"] == "inProgress")
                {
                    MessageStatus::Interrupted
                } else {
                    MessageStatus::Complete
                },
                error: if failed {
                    turn["error"]["message"].as_str().map(str::to_owned)
                } else {
                    None
                },
                model: model.to_owned(),
                attachments: Vec::new(),
                activity,
            });
        }
        if let Some(index) = last_reply {
            let last = &mut messages[index];
            last.activity.turn_id = turn_id.to_owned();
            for mut command in activity.commands {
                command.offset = last.content.len();
                last.activity.record_command(command);
            }
        }
    }
    Ok(messages)
}

pub fn import(session: &mut Session, page: ConversationPage, older: bool) -> Uuid {
    let locally_archived = session.local_archive_overrides.get(&page.entry.id).copied();
    let index = session
        .chats
        .iter()
        .position(|chat| chat.thread_id.as_deref() == Some(&page.entry.id))
        .unwrap_or_else(|| {
            let mut chat = Chat::new(Some(page.entry.cwd.clone()));
            chat.imported_history = true;
            chat.thread_id = Some(page.entry.id.clone());
            session.chats.push(chat);
            session.chats.len() - 1
        });
    let chat = &mut session.chats[index];
    if older {
        let mut messages = page.messages;
        messages.append(&mut chat.messages);
        chat.messages = messages;
    } else {
        chat.title = page.entry.title();
        if chat.title.is_empty() {
            chat.title = "Codex conversation".into();
        }
        chat.archived = locally_archived.unwrap_or(page.archived);
        let mut messages = page.messages;
        for mut local in chat.messages.drain(..) {
            if local.role == Role::User
                && matches!(
                    local.status,
                    MessageStatus::Queued | MessageStatus::Interrupted
                )
            {
                messages.push(local);
            } else if !local.activity.turn_id.is_empty()
                && let Some(imported) = messages
                    .iter_mut()
                    .find(|message| message.activity.turn_id == local.activity.turn_id)
            {
                imported.activity.files = std::mem::take(&mut local.activity.files);
                imported.activity.preview_limited = local.activity.preview_limited;
            }
        }
        chat.messages = messages;
        if chat.archived {
            chat.interrupt_queue("Chat was archived before this message was sent.");
        }
    }
    chat.id
}
