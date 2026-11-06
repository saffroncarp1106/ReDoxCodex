use super::*;

#[test]
fn repeated_new_chat_reuses_the_blank_and_preserves_unsent_drafts() {
    let mut session = Session::default();
    session.new_chat();
    let first = session.active_chat;
    for _ in 0..5 {
        session.new_chat();
    }
    assert_eq!(session.chats.len(), 1);
    assert_eq!(session.active_chat, first);
    session.active_mut().unwrap().draft = "Unsent work".into();
    session.new_chat();
    let blank = session.active_chat;
    assert_ne!(blank, first);
    session.active_chat = first;
    session.new_chat();
    assert_eq!(session.active_chat, blank);
    assert_eq!(session.chats.len(), 2);
    assert_eq!(
        session
            .chats
            .iter()
            .find(|chat| Some(chat.id) == first)
            .unwrap()
            .draft,
        "Unsent work"
    );
}

#[test]
fn blank_chat_reuse_respects_project_and_provider() {
    let mut session = Session {
        workspace: Some("project-one".into()),
        ..Default::default()
    };
    session.new_chat();
    let one = session.active_chat;
    session.workspace = Some("project-two".into());
    session.new_chat();
    assert_ne!(one, session.active_chat);
    session
        .active_mut()
        .unwrap()
        .set_provider(Provider::Ollama, "qwen-test".into());
    for _ in 0..3 {
        session.new_chat();
    }
    assert_eq!(session.chats.len(), 2);
    assert_eq!(session.active().unwrap().provider, Provider::Ollama);
    assert_eq!(session.active().unwrap().ollama_model, "qwen-test");
}

fn page() -> crate::history::ConversationPage {
    crate::history::ConversationPage {
        entry: crate::history::HistoryEntry {
            id: "server-thread".into(),
            name: Some("Saved conversation".into()),
            preview: String::new(),
            cwd: "project".into(),
            model: None,
        },
        messages: Vec::new(),
        next_cursor: None,
        archived: false,
    }
}

#[test]
fn local_archiving_survives_history_reload_and_restores_without_server_mutation() {
    let mut session = Session::default();
    let id = crate::history::import(&mut session, page(), false);
    session.active_chat = Some(id);
    assert!(session.set_chat_archived(id, true));
    let reloaded = crate::history::import(&mut session, page(), false);
    assert_eq!(id, reloaded);
    assert!(session.active().unwrap().archived);
    let mut restored: Session =
        serde_json::from_slice(&serde_json::to_vec(&session).unwrap()).unwrap();
    restored.recover();
    assert!(restored.active().unwrap().archived);
    assert!(restored.set_chat_archived(id, false));
    crate::history::import(&mut restored, page(), false);
    assert!(!restored.active().unwrap().archived);
    assert_eq!(
        restored.active().unwrap().thread_id.as_deref(),
        Some("server-thread")
    );
}

#[test]
fn running_chats_cannot_be_archived_and_archived_blanks_are_not_reused() {
    let mut session = Session::default();
    session.new_chat();
    session.active_mut().unwrap().draft = "Test".into();
    let run = session.prepare_run().unwrap();
    assert!(!session.set_chat_archived(run.chat, true));
    session.finish_run(run, MessageStatus::Complete, None);
    assert!(session.set_chat_archived(run.chat, true));
    session.new_chat();
    assert_ne!(session.active_chat, Some(run.chat));
    assert!(session.active().unwrap().is_blank());
}
