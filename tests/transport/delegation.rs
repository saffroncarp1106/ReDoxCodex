use super::*;
use crate::domain::{Provider, Session};
use crate::run::{RunEvent, Update};

fn task(id: &str) -> Task {
    Task {
        id: id.into(),
        model: "qwen-test".into(),
        task: "Read the project".into(),
        status: Status::Running,
        progress: "Starting".into(),
        output: String::new(),
        error: None,
    }
}

#[tokio::test]
async fn worker_results_return_to_codex_while_progress_and_usage_stay_visible_in_agents() {
    let (endpoint, captured, handle) = ollama::tests::server(vec![
        ("/api/show", json!({"capabilities":[]}).to_string()),
        (
            "/api/chat",
            format!(
                "{}\n",
                json!({"message":{"content":"LOCAL_REVIEW_RESULT"},"done":true,"prompt_eval_count":11,"eval_count":4})
            ),
        ),
    ]);
    let settings = ollama::Settings {
        endpoint,
        preferred_model: "selected-qwen".into(),
        ..ollama::Settings::default()
    };
    let (sender, mut events) = mpsc::channel(32);
    let (_cancel, receiver) = watch::channel(false);
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), execute(
        &json!({"task":"Review the project","context":"Check naming","model":"ignored-unselected-model"}),
        settings, None, Mode::Chat, sender, receiver)).await.unwrap().unwrap();
    handle.join().unwrap();
    assert!(result.contains("LOCAL_REVIEW_RESULT"));
    assert_eq!(tool_result(Ok(result))["success"], true);
    let mut tasks = Vec::new();
    let mut tokens = 0;
    while let Some(event) = events.recv().await {
        match event {
            ProviderEvent::Delegation(event) => apply(&mut tasks, event),
            ProviderEvent::Metrics(stats) => tokens += stats.output_tokens,
            ProviderEvent::Content(_) => panic!(
                "Local output must return to Codex as a tool result, not impersonate its chat response"
            ),
            _ => {}
        }
    }
    assert_eq!(tokens, 4);
    assert_eq!(tasks[0].model, "selected-qwen");
    assert_eq!(tasks[0].output, "LOCAL_REVIEW_RESULT");
    assert_eq!(tasks[0].status, Status::Complete);
    assert_eq!(captured.lock().unwrap()[1].1["model"], "selected-qwen");
}

#[tokio::test]
async fn delegated_project_changes_use_the_parent_approval_flow_and_access_mode() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("temp/verification/delegation")
        .join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&root).unwrap();
    let call = json!({"message":{"content":"","tool_calls":[{"function":{"name":"write_file","arguments":{"path":"new.txt","content":"unapproved"}}}]},"done":true});
    let (endpoint, captured, handle) = ollama::tests::server(vec![
        ("/api/show", json!({"capabilities":["tools"]}).to_string()),
        ("/api/chat", format!("{call}\n")),
        (
            "/api/chat",
            format!(
                "{}\n",
                json!({"message":{"content":"Change rejected by the user."},"done":true})
            ),
        ),
    ]);
    let settings = ollama::Settings {
        endpoint,
        preferred_model: "qwen-test".into(),
        ..ollama::Settings::default()
    };
    let (sender, mut events) = mpsc::channel(32);
    let (_cancel, receiver) = watch::channel(false);
    let observe = async {
        let mut approval_count = 0;
        while let Some(event) = events.recv().await {
            if let ProviderEvent::Approval(approval) = event {
                assert!(approval.description.contains("new.txt"));
                approval.response.send(false).unwrap();
                approval_count += 1;
            }
        }
        approval_count
    };
    let args = json!({"task":"Create a file"});
    let (result, count) = tokio::join!(
        execute(
            &args,
            settings,
            Some(root.clone()),
            Mode::Code,
            sender,
            receiver
        ),
        observe
    );
    assert!(result.is_ok());
    assert_eq!(count, 1);
    assert!(!root.join("new.txt").exists());
    handle.join().unwrap();
    assert!(
        captured.lock().unwrap()[2].1["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|message| message["role"] == "tool"
                && message["content"].as_str().unwrap().contains("rejected"))
    );
}

#[test]
fn codex_conversations_get_a_fresh_tool_enabled_thread_without_changing_provider() {
    let mut session = Session::default();
    session.new_chat();
    session.ollama.preferred_model = "qwen-test".into();
    let chat = session.active_mut().unwrap();
    chat.thread_id = Some("old-thread-without-tool".into());
    chat.draft = "Delegate a review".into();
    let run = session.prepare_run().unwrap();
    assert_eq!(session.active().unwrap().provider, Provider::Codex);
    assert!(session.active().unwrap().thread_id.is_none());
    assert!(
        session
            .active()
            .unwrap()
            .previous_thread_ids
            .contains(&"old-thread-without-tool".into())
    );
    crate::run::apply_event(
        &mut session,
        Some(run),
        RunEvent {
            run,
            update: Update::Thread("tool-enabled-thread".into()),
        },
    );
    assert!(session.active().unwrap().delegation_tools);
    session.finish_run(run, crate::domain::MessageStatus::Complete, None);
    session.active_mut().unwrap().draft = "Continue".into();
    session.prepare_run().unwrap();
    assert_eq!(
        session.active().unwrap().thread_id.as_deref(),
        Some("tool-enabled-thread")
    );
}

#[test]
fn progress_output_and_cancellation_are_scoped_to_the_parent_run() {
    let mut session = Session::default();
    session.new_chat();
    session.active_mut().unwrap().draft = "Delegate".into();
    let run = session.prepare_run().unwrap();
    crate::run::apply_event(
        &mut session,
        Some(run),
        RunEvent {
            run,
            update: Update::Delegation(Event::Started(task("one"))),
        },
    );
    crate::run::apply_event(
        &mut session,
        Some(run),
        RunEvent {
            run,
            update: Update::Delegation(Event::Content {
                id: "one".into(),
                text: "Local result".into(),
            }),
        },
    );
    assert_eq!(
        session.active().unwrap().delegated_tasks[0].output,
        "Local result"
    );
    assert_eq!(
        session.active().unwrap().messages.last().unwrap().content,
        ""
    );
    session.finish_run(run, crate::domain::MessageStatus::Cancelled, None);
    assert_eq!(
        session.active().unwrap().delegated_tasks[0].status,
        Status::Cancelled
    );
    crate::run::apply_event(
        &mut session,
        None,
        RunEvent {
            run,
            update: Update::Delegation(Event::Content {
                id: "one".into(),
                text: "stale".into(),
            }),
        },
    );
    assert_eq!(
        session.active().unwrap().delegated_tasks[0].output,
        "Local result"
    );
}

#[test]
fn interrupted_worker_logs_survive_restart_and_tool_responses_preserve_failure() {
    let mut session = Session::default();
    session.new_chat();
    session
        .active_mut()
        .unwrap()
        .delegated_tasks
        .push(task("one"));
    let mut restored: Session =
        serde_json::from_slice(&serde_json::to_vec(&session).unwrap()).unwrap();
    restored.recover();
    assert_eq!(
        restored.active().unwrap().delegated_tasks[0].status,
        Status::Interrupted
    );
    assert_eq!(tool_spec()["name"], TOOL_NAME);
    assert_eq!(tool_spec()["inputSchema"]["additionalProperties"], false);
    let result = tool_result(Err("Server unavailable".into()));
    assert_eq!(result["success"], false);
    assert!(
        result["contentItems"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Server unavailable")
    );
    let mut text = String::new();
    append_bounded(&mut text, &"é".repeat(OUTPUT_LIMIT));
    assert_eq!(text.len(), OUTPUT_LIMIT);
}

#[tokio::test]
async fn disabled_delegation_does_not_contact_an_ollama_server() {
    let settings = ollama::Settings {
        endpoint: "http://127.0.0.1:1".into(),
        preferred_model: "qwen-test".into(),
        codex_delegation: false,
        ..ollama::Settings::default()
    };
    let (sender, mut events) = mpsc::channel(16);
    let (_cancel, receiver) = watch::channel(false);
    let error = execute(
        &json!({"task":"test"}),
        settings,
        None,
        Mode::Chat,
        sender,
        receiver,
    )
    .await
    .unwrap_err();
    assert!(error.contains("enable Codex delegation"));
    assert!(events.recv().await.is_none());
}

#[tokio::test]
#[ignore = "read-only installed Codex protocol check; does not start a model turn"]
async fn installed_codex_accepts_the_delegation_tool_without_a_generation() {
    let mut rpc = crate::codex::Rpc::connect(&crate::codex::find_executable())
        .await
        .unwrap();
    let result = rpc.call("thread/start", json!({"ephemeral":true,"sandbox":"read-only","approvalPolicy":"untrusted","dynamicTools":[tool_spec()]})).await;
    rpc.close().await;
    let result = result.unwrap();
    assert!(result["thread"]["id"].is_string());
}
