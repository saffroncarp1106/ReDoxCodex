use super::*;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

pub(crate) type Requests = Arc<Mutex<Vec<(String, Value)>>>;

pub(crate) fn server(
    responses: Vec<(&'static str, String)>,
) -> (String, Requests, std::thread::JoinHandle<()>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let handle = std::thread::spawn(move || {
        for (expected_path, body) in responses {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            let end = loop {
                let mut byte = [0];
                socket.read_exact(&mut byte).unwrap();
                bytes.push(byte[0]);
                if bytes.ends_with(b"\r\n\r\n") {
                    break bytes.len();
                }
                assert!(bytes.len() < 16384);
            };
            let headers = String::from_utf8(bytes[..end].to_vec()).unwrap();
            assert!(
                headers.lines().next().unwrap().contains(expected_path),
                "{headers}"
            );
            let length: usize = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|length| length.trim().parse().unwrap())
                })
                .unwrap_or(0);
            let mut payload = vec![0; length];
            socket.read_exact(&mut payload).unwrap();
            captured.lock().unwrap().push((
                expected_path.into(),
                serde_json::from_slice(&payload).unwrap_or(Value::Null),
            ));
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
            for chunk in body.as_bytes().chunks(3) {
                socket.write_all(chunk).unwrap();
            }
        }
    });
    (endpoint, requests, handle)
}

fn request(endpoint: String, tools: bool) -> Request {
    let mut chat = Chat::new(None);
    chat.set_provider(crate::domain::Provider::Ollama, "qwen-test:latest".into());
    chat.draft = "test".into();
    Request {
        settings: Settings {
            endpoint,
            project_tools: tools,
            ..Settings::default()
        },
        model: chat.ollama_model,
        workspace: None,
        mode: Mode::Chat,
        desktop: Default::default(),
        desktop_activity: Default::default(),
        messages: vec![InputMessage {
            role: "user",
            content: "Hello".into(),
            images: Vec::new(),
        }],
    }
}

#[tokio::test]
async fn discovery_and_fragmented_streams_use_native_ollama_routes_and_record_usage() {
    let body = "{\"message\":{\"content\":\"Hé\"},\"done\":false}\n{\"message\":{\"content\":\"llo\"},\"done\":false}\n{\"message\":{\"content\":\"\"},\"done\":true,\"prompt_eval_count\":7,\"eval_count\":3,\"eval_duration\":1000000000}";
    let (endpoint, captured, handle) = server(vec![
        (
            "/api/tags",
            json!({"models":[{"name":"qwen-test:latest"}]}).to_string(),
        ),
        (
            "/api/show",
            json!({"capabilities":["completion"]}).to_string(),
        ),
        ("/api/chat", body.into()),
    ]);
    assert_eq!(
        list_models(&endpoint).await.unwrap()[0].name,
        "qwen-test:latest"
    );
    let (sender, mut events) = mpsc::channel(16);
    let (_cancel, receiver) = watch::channel(false);
    let mut local_request = request(endpoint, false);
    local_request.settings.cpu_only = true;
    stream_chat(local_request, sender, receiver).await.unwrap();
    let mut text = String::new();
    let mut usage = Usage::default();
    while let Some(event) = events.recv().await {
        match event {
            ProviderEvent::Content(content) => text.push_str(&content),
            ProviderEvent::Metrics(stats) => usage.record(stats),
            _ => panic!("Unexpected provider event"),
        }
    }
    assert_eq!(text, "Héllo");
    assert_eq!(usage.prompt_tokens, 7);
    assert_eq!(usage.output_tokens, 3);
    assert_eq!(usage.last.eval_seconds, 1.0);
    handle.join().unwrap();
    let requests = captured.lock().unwrap();
    assert_eq!(requests[2].1["model"], "qwen-test:latest");
    assert_eq!(requests[2].1["messages"][1]["role"], "user");
    assert!(requests[2].1.get("tools").is_none());
    assert_eq!(requests[2].1["options"]["num_gpu"], 0);
}

#[tokio::test]
async fn local_agent_executes_a_project_tool_and_supplies_its_result_to_the_model() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("temp/verification/ollama-stream")
        .join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("note.txt"), "PROJECT_DATA_123").unwrap();
    let call = json!({"message":{"role":"assistant","content":"","tool_calls":[{"function":{"name":"read_file","arguments":{"path":"note.txt"}}}]},"done":true,"eval_count":2});
    let final_reply = json!({"message":{"content":"Read the project."},"done":true,"eval_count":4});
    let (endpoint, captured, handle) = server(vec![
        ("/api/show", json!({"capabilities":["tools"]}).to_string()),
        ("/api/chat", format!("{call}\n")),
        ("/api/chat", format!("{final_reply}\n")),
    ]);
    let mut request = request(endpoint, true);
    request.workspace = Some(root);
    let (sender, mut events) = mpsc::channel(16);
    let (_cancel, receiver) = watch::channel(false);
    stream_chat(request, sender, receiver).await.unwrap();
    handle.join().unwrap();
    {
        let calls = captured.lock().unwrap();
        let messages = calls[2].1["messages"].as_array().unwrap();
        assert!(
            messages.iter().any(
                |message| message["role"] == "tool" && message["content"] == "PROJECT_DATA_123"
            )
        );
    }
    let mut stats = None;
    while let Some(event) = events.recv().await {
        if let ProviderEvent::Metrics(value) = event {
            stats = Some(value);
        }
    }
    assert_eq!(stats.unwrap().output_tokens, 6);
}

#[tokio::test]
async fn disconnected_streams_fail_and_pre_cancelled_requests_do_not_connect() {
    let (endpoint, _, handle) = server(vec![
        ("/api/show", json!({"capabilities":[]}).to_string()),
        (
            "/api/chat",
            "{\"message\":{\"content\":\"partial\"},\"done\":false}\n".into(),
        ),
    ]);
    let (sender, mut events) = mpsc::channel(16);
    let (_cancel, receiver) = watch::channel(false);
    assert!(
        stream_chat(request(endpoint, false), sender, receiver)
            .await
            .unwrap_err()
            .contains("disconnected")
    );
    assert!(matches!(events.recv().await, Some(ProviderEvent::Content(text)) if text == "partial"));
    handle.join().unwrap();
    let (sender, _) = mpsc::channel(16);
    let (_cancel, receiver) = watch::channel(true);
    assert_eq!(
        stream_chat(
            request("http://127.0.0.1:1".into(), false),
            sender,
            receiver
        )
        .await
        .unwrap_err(),
        "Cancelled."
    );
}

#[cfg(windows)]
#[tokio::test]
async fn folderless_ollama_desktop_input_uses_shared_approval() {
    let call = json!({"message":{"role":"assistant","tool_calls":[{
        "function":{"name":"desktop_control","arguments":{"action":"keypress","keys":["ESC"]}}
    }]},"done":true});
    let reply = json!({"message":{"content":"Desktop input was declined."},"done":true});
    let (endpoint, captured, handle) = server(vec![
        (
            "/api/show",
            json!({"capabilities":["vision","tools"]}).to_string(),
        ),
        ("/api/chat", format!("{call}\n")),
        ("/api/chat", format!("{reply}\n")),
    ]);
    let mut request = request(endpoint, false);
    request.desktop.enabled = true;
    let activity = request.desktop_activity.clone();
    let (sender, mut events) = mpsc::channel(16);
    let (_cancel, receiver) = watch::channel(false);
    let worker = tokio::spawn(stream_chat(request, sender, receiver));
    while let Some(event) = events.recv().await {
        if let ProviderEvent::Approval(approval) = event {
            approval.response.send(false).unwrap();
        }
    }
    worker.await.unwrap().unwrap();
    handle.join().unwrap();
    let requests = captured.lock().unwrap();
    let tools = requests[1].1["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["function"]["name"], "desktop_control");
    assert!(
        requests[2].1["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|message| {
                message["role"] == "tool"
                    && message["content"]
                        .as_str()
                        .unwrap()
                        .contains("not approved")
            })
    );
    assert!(!activity.snapshot().active);
}

#[test]
fn desktop_screenshot_is_an_ollama_tool_message_with_an_image() {
    let message = desktop_tool_message(Ok(vec![
        json!({"type":"inputText","text":"Origin (-1920, 0)"}),
        json!({"type":"inputImage","imageUrl":"data:image/jpeg;base64,YWJj"}),
    ]));
    assert_eq!(message["role"], "tool");
    assert_eq!(message["tool_name"], "desktop_control");
    assert_eq!(message["content"], "Origin (-1920, 0)");
    assert_eq!(message["images"], json!(["YWJj"]));
}

#[test]
fn local_chats_survive_restart_and_switching_provider_starts_a_fresh_codex_thread() {
    let mut session = crate::domain::Session::default();
    session.new_chat();
    let chat = session.active_mut().unwrap();
    chat.thread_id = Some("old-codex-thread".into());
    chat.set_provider(crate::domain::Provider::Ollama, "qwen3.5:9b".into());
    chat.draft = "Offload this request".into();
    assert!(chat.thread_id.is_none());
    let id = session.prepare_run().unwrap();
    assert!(
        session
            .run_message_mut(id)
            .unwrap()
            .model
            .starts_with("Ollama ·")
    );
    let saved = serde_json::to_vec(&session).unwrap();
    let mut restored: crate::domain::Session = serde_json::from_slice(&saved).unwrap();
    restored.recover();
    assert_eq!(restored.active().unwrap().ollama_model, "qwen3.5:9b");
    assert_eq!(
        restored.active().unwrap().provider,
        crate::domain::Provider::Ollama
    );
    assert_eq!(
        restored.active().unwrap().messages.last().unwrap().status,
        MessageStatus::Interrupted
    );
    assert!(api_url("file:///project", "chat").is_err());
    assert_eq!(
        api_url("http://localhost:11434/api/", "chat")
            .unwrap()
            .path(),
        "/api/chat"
    );
}

#[tokio::test]
#[ignore = "uses an installed local Ollama model; no model download or Codex request"]
async fn live_local_model_smoke_test() {
    let model = std::env::var("REDOX_OLLAMA_TEST_MODEL").expect("Set REDOX_OLLAMA_TEST_MODEL");
    let mut request = request("http://127.0.0.1:11434".into(), false);
    request.model = model;
    request.settings.output_tokens = 16;
    request.settings.cpu_only =
        std::env::var("REDOX_OLLAMA_TEST_CPU").is_ok_and(|value| value == "1");
    request.messages[0].content = "Reply with exactly LOCAL_OK".into();
    let (sender, mut events) = mpsc::channel(128);
    let (_cancel, receiver) = watch::channel(false);
    tokio::time::timeout(
        Duration::from_secs(120),
        stream_chat(request, sender, receiver),
    )
    .await
    .unwrap()
    .unwrap();
    let mut text = String::new();
    let mut stats = None;
    while let Some(event) = events.recv().await {
        match event {
            ProviderEvent::Content(content) => text.push_str(&content),
            ProviderEvent::Metrics(value) => stats = Some(value),
            _ => {}
        }
    }
    assert!(text.contains("LOCAL_OK"), "{text}");
    let stats = stats.expect("No usage metrics");
    assert!(stats.output_tokens > 0);
    println!(
        "Local Ollama replied {text}; {} output tokens",
        stats.output_tokens
    );
}

#[tokio::test]
#[ignore = "uses an installed local Ollama model to read a temporary project fixture"]
async fn live_local_project_read_test() {
    let model = std::env::var("REDOX_OLLAMA_TEST_MODEL").expect("Set REDOX_OLLAMA_TEST_MODEL");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("temp/verification/ollama-live-project")
        .join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&root).unwrap();
    let proof = format!("LOCAL_TOOL_{}", uuid::Uuid::new_v4());
    std::fs::write(root.join("proof.txt"), &proof).unwrap();
    let mut request = request("http://127.0.0.1:11434".into(), true);
    request.model = model;
    request.workspace = Some(root);
    request.settings.cpu_only = true;
    request.settings.output_tokens = 128;
    request.messages[0].content = "Use the read_file tool to read proof.txt in the selected project. Reply with only its exact contents. Do not guess the contents.".into();
    let (sender, mut events) = mpsc::channel(256);
    let (_cancel, receiver) = watch::channel(false);
    tokio::time::timeout(
        Duration::from_secs(120),
        stream_chat(request, sender, receiver),
    )
    .await
    .unwrap()
    .unwrap();
    let mut text = String::new();
    let mut read_tool = false;
    let mut stats = None;
    while let Some(event) = events.recv().await {
        match event {
            ProviderEvent::Content(content) => text.push_str(&content),
            ProviderEvent::Status(status) => read_tool |= status.contains("read_file"),
            ProviderEvent::Metrics(value) => stats = Some(value),
            _ => {}
        }
    }
    assert!(read_tool, "The model did not use the file tool");
    assert!(
        text.contains(&proof),
        "The model did not return the actual file content: {text}"
    );
    println!(
        "Local Qwen read a random project fixture using read_file; {} total output tokens",
        stats.unwrap().output_tokens
    );
}
