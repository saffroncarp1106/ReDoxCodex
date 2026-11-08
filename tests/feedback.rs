use redox::codex::{ChatRequest, Feedback};
use redox::domain::{MessageStatus, Session};
use redox::run::{RunEvent, Update, apply_event};
use std::time::{Duration, Instant};

#[test]
fn scheduled_prompt_waits_without_blocking_a_manual_send_or_losing_a_draft() {
    let mut session = Session::default();
    session.new_chat();
    session.active_mut().unwrap().draft = "Scheduled".into();
    let delayed = session
        .queue_prompt_after(Duration::from_secs(600))
        .unwrap();
    let now = Instant::now();
    assert_eq!(session.active().unwrap().ready_queued(now), None);
    assert!(session.prepare_selected_run(Some(delayed)).is_err());

    session.active_mut().unwrap().draft = "Send now".into();
    let run = session.prepare_selected_run(None).unwrap();
    assert!(
        ChatRequest::from_chat(session.active().unwrap(), "test".into())
            .prompt
            .contains("Send now")
    );
    session.finish_run(run, MessageStatus::Complete, None);

    let chat = session.active_mut().unwrap();
    chat.draft = "Keep this draft".into();
    chat.send_after.insert(delayed, now);
    assert_eq!(chat.ready_queued(now), Some(delayed));
    session.prepare_selected_run(Some(delayed)).unwrap();
    let chat = session.active().unwrap();
    assert!(chat.send_after.is_empty());
    assert_eq!(chat.draft, "Keep this draft");
    assert_eq!(
        chat.messages
            .iter()
            .find(|m| m.id == delayed)
            .unwrap()
            .status,
        MessageStatus::Complete
    );
}

#[test]
fn closing_redox_preserves_the_queued_text_without_auto_sending_on_restart() {
    let mut session = Session::default();
    session.new_chat();
    session.active_mut().unwrap().draft = "Do this later".into();
    session
        .queue_prompt_after(Duration::from_secs(600))
        .unwrap();
    let mut restored: Session =
        serde_json::from_str(&serde_json::to_string(&session).unwrap()).unwrap();
    restored.recover();
    let chat = restored.active().unwrap();
    assert_eq!(chat.ready_queued(Instant::now()), None);
    assert_eq!(chat.messages[0].status, MessageStatus::Interrupted);
    assert_eq!(chat.messages[0].content, "Do this later");
}

#[test]
fn project_branch_copies_completed_context_and_keeps_a_separate_chat() {
    let project = std::path::Path::new("project");
    let mut session = Session {
        workspace: Some(project.into()),
        ..Default::default()
    };
    session.new_project_chat(project.into());
    let original = session.active_chat.unwrap();
    session.active_mut().unwrap().draft = "Original question".into();
    let run = session.prepare_run().unwrap();
    session.run_message_mut(run).unwrap().content = "Original answer".into();
    session.finish_run(run, MessageStatus::Complete, None);
    let source = session.active_mut().unwrap();
    source.thread_id = Some("original-thread".into());
    source.draft = "Private unsent draft".into();
    session.queue_prompt().unwrap();
    session.active_mut().unwrap().draft = "New unsent draft".into();

    let branch = session.branch_project_chat(project);
    assert_eq!(session.active_chat, Some(original));
    assert_ne!(branch, original);
    assert_eq!(session.active().unwrap().draft, "New unsent draft");
    let chat = session.chats.iter().find(|chat| chat.id == branch).unwrap();
    assert!(chat.thread_id.is_none());
    assert_eq!(chat.workspace.as_deref(), Some(project));
    assert!(chat.draft.is_empty());
    assert_eq!(
        chat.messages
            .iter()
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>(),
        ["Original question", "Original answer"]
    );

    session.active_chat = Some(branch);
    session.active_mut().unwrap().draft = "Branch followup".into();
    session.prepare_run().unwrap();
    assert!(
        ChatRequest::from_chat(session.active().unwrap(), "test".into())
            .prompt
            .contains("Branch followup")
    );
    assert_eq!(
        session
            .chats
            .iter()
            .find(|chat| chat.id == original)
            .unwrap()
            .messages
            .len(),
        3
    );
}

#[test]
fn multiple_followups_preserve_the_running_response_and_unsent_draft() {
    let mut session = Session::default();
    session.new_chat();
    session.active_mut().unwrap().draft = "First task".into();
    let run = session.prepare_run().unwrap();
    for text in ["First correction", "Second correction"] {
        session.active_mut().unwrap().draft = text.into();
        session.queue_prompt().unwrap();
    }
    assert!(session.run_message_mut(run).is_some());
    let saved = serde_json::to_string(&session).unwrap();
    let mut session: Session = serde_json::from_str(&saved).unwrap();
    assert_eq!(
        session
            .active()
            .unwrap()
            .messages
            .iter()
            .filter(|m| m.status == MessageStatus::Queued)
            .count(),
        2
    );
    session.finish_run(run, MessageStatus::Complete, None);
    session.active_mut().unwrap().draft = "Still typing this".into();
    let next = session.prepare_run().unwrap();
    let request = ChatRequest::from_chat(session.active().unwrap(), "test".into());
    assert!(request.prompt.contains("First correction"));
    assert!(!request.prompt.contains("Second correction"));
    assert_eq!(session.active().unwrap().draft, "Still typing this");
    session.finish_run(next, MessageStatus::Complete, None);
    session.prepare_run().unwrap();
    let request = ChatRequest::from_chat(session.active().unwrap(), "test".into());
    assert!(request.prompt.contains("Second correction"));
}

#[test]
fn accepted_feedback_is_not_queued_again_and_stop_preserves_unsent_feedback() {
    let mut session = Session::default();
    session.new_chat();
    session.active_mut().unwrap().draft = "Task".into();
    let run = session.prepare_run().unwrap();
    session.active_mut().unwrap().draft = "Correction".into();
    let id = session.queue_prompt().unwrap();
    let input = Feedback::from_message(session.active().unwrap().messages.last().unwrap());
    let params = input.params("thread", "active-turn");
    assert_eq!(params["expectedTurnId"], "active-turn");
    assert_eq!(params["input"][0]["text"], "Correction");
    assert!(params.get("effort").is_none());
    apply_event(
        &mut session,
        Some(run),
        RunEvent {
            run,
            update: Update::Feedback(id, Ok(())),
        },
    );
    assert_eq!(
        session.active().unwrap().messages.last().unwrap().status,
        MessageStatus::Complete
    );
    session.active_mut().unwrap().draft = "Not yet accepted".into();
    let pending = session.queue_prompt().unwrap();
    session.stop_queued(run.chat);
    let message = session
        .active()
        .unwrap()
        .messages
        .iter()
        .find(|m| m.id == pending)
        .unwrap();
    assert_eq!(message.status, MessageStatus::Interrupted);
    assert_eq!(message.content, "Not yet accepted");
    assert!(message.error.is_some());
}

#[test]
fn rejected_steering_remains_queued_without_failing_the_active_run() {
    let mut session = Session::default();
    session.new_chat();
    session.active_mut().unwrap().draft = "Task".into();
    let run = session.prepare_run().unwrap();
    session.active_mut().unwrap().draft = "Followup".into();
    let id = session.queue_prompt().unwrap();
    assert!(!apply_event(
        &mut session,
        Some(run),
        RunEvent {
            run,
            update: Update::Feedback(id, Err("No active turn".into()))
        }
    ));
    assert!(session.run_message_mut(run).is_some());
    assert_eq!(
        session.active().unwrap().messages.last().unwrap().status,
        MessageStatus::Queued
    );
}
