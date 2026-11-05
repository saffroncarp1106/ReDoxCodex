use eframe::egui;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

use crate::codex::{self, Approval, ChatRequest, ProviderEvent};
use crate::domain::{MessageStatus, RunId, Session};

pub enum Update {
    Content(String),
    Complete,
    Failed(String),
    Thread(String),
    Status(String),
    Approval(Approval),
}

pub struct RunEvent {
    pub run: RunId,
    pub update: Update,
}

pub struct Running {
    pub id: RunId,
    task: JoinHandle<()>,
    cancel: watch::Sender<bool>,
}

impl Running {
    pub fn start(
        runtime: &tokio::runtime::Runtime,
        id: RunId,
        request: ChatRequest,
        executable: String,
        sender: mpsc::Sender<RunEvent>,
        context: egui::Context,
    ) -> Self {
        let (cancel, cancellation) = watch::channel(false);
        let task = runtime.spawn(async move {
            let (provider_sender, mut provider_receiver) = mpsc::channel(128);
            let producer = tokio::spawn(async move {
                codex::stream_chat(&executable, request, provider_sender, cancellation).await
            });

            while let Some(event) = provider_receiver.recv().await {
                let update = match event {
                    ProviderEvent::Thread(thread) => Update::Thread(thread),
                    ProviderEvent::Content(text) => Update::Content(text),
                    ProviderEvent::Status(text) => Update::Status(text),
                    ProviderEvent::Approval(approval) => Update::Approval(approval),
                };

                if sender.send(RunEvent { run: id, update }).await.is_err() {
                    producer.abort();
                    return;
                }

                context.request_repaint();
            }

            let result = producer
                .await
                .unwrap_or_else(|error| Err(format!("Codex worker stopped: {error}")));

            let update = match result {
                Ok(()) => Update::Complete,
                Err(error) => Update::Failed(error),
            };

            let _ = sender.send(RunEvent { run: id, update }).await;
            context.request_repaint();
        });

        Self { id, task, cancel }
    }

    pub fn cancel(&self) {
        let _ = self.cancel.send(true);
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.cancel.send(true);
        if self.task.is_finished() {
            self.task.abort();
        }
    }
}

pub fn apply_event(session: &mut Session, active: Option<RunId>, event: RunEvent) -> bool {
    if active != Some(event.run) || session.run_message_mut(event.run).is_none() {
        return false;
    }

    match event.update {
        Update::Thread(thread) => {
            if let Some(chat) = session
                .chats
                .iter_mut()
                .find(|chat| chat.id == event.run.chat)
            {
                chat.thread_id = Some(thread);
            }
            false
        }
        Update::Status(_) | Update::Approval(_) => false,
        Update::Content(text) => {
            if let Some(message) = session.run_message_mut(event.run) {
                message.content.push_str(&text);
            }
            false
        }
        Update::Complete => {
            session.finish_run(event.run, MessageStatus::Complete, None);
            true
        }
        Update::Failed(error) => {
            session.finish_run(event.run, MessageStatus::Failed, Some(error));
            true
        }
    }
}
