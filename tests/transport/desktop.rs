use super::*;
use crate::domain::{MessageStatus, Session};

#[test]
fn tool_registration_composes_with_ollama_and_settings_survive_restart() {
    let mut session = Session::default();
    session.new_chat();
    let mut request =
        crate::codex::ChatRequest::from_chat(session.active().unwrap(), "default".into());
    assert!(crate::codex::dynamic_tools(&request).is_empty());
    request.desktop.enabled = true;
    request.delegation = Some(crate::ollama::Settings::default());
    let tools = crate::codex::dynamic_tools(&request);
    assert_eq!(tools.len(), if SUPPORTED { 2 } else { 1 });
    assert_eq!(tools[0]["name"], crate::delegation::TOOL_NAME);
    if SUPPORTED {
        assert_eq!(tools[1]["name"], TOOL_NAME);
    }
    session.desktop = request.desktop;
    let restored: Session = serde_json::from_slice(&serde_json::to_vec(&session).unwrap()).unwrap();
    assert!(restored.desktop.enabled);
    assert!(restored.desktop.confirm_actions);
}

#[test]
fn enabling_and_disabling_desktop_refreshes_server_tools_without_losing_history() {
    if !SUPPORTED {
        return;
    }
    let mut session = Session::default();
    session.new_chat();
    let chat = session.active_mut().unwrap();
    chat.thread_id = Some("old-thread".into());
    chat.draft = "First message".into();
    session.desktop.enabled = true;
    let run = session.prepare_run().unwrap();
    assert!(session.active().unwrap().thread_id.is_none());
    assert_eq!(
        session.active().unwrap().previous_thread_ids,
        ["old-thread"]
    );
    crate::run::apply_event(
        &mut session,
        Some(run),
        crate::run::RunEvent {
            run,
            update: crate::run::Update::Thread("desktop-thread".into()),
        },
    );
    assert!(session.active().unwrap().desktop_tools);
    session.finish_run(run, MessageStatus::Complete, None);
    session.desktop.enabled = false;
    session.active_mut().unwrap().draft = "Next message".into();
    session.prepare_run().unwrap();
    assert!(session.active().unwrap().thread_id.is_none());
    assert_eq!(session.active().unwrap().messages.len(), 4);
}

#[test]
fn stale_project_folders_fail_before_a_command_starts_and_can_be_relocated() {
    let missing = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let error = crate::codex::validate_workspace(Some(&missing)).unwrap_err();
    assert!(error.contains("moved or been renamed"));
    assert!(error.contains("No command was started"));
    let valid = std::env::current_dir().unwrap();
    assert!(crate::codex::validate_workspace(Some(&valid)).is_ok());
    let mut chat = crate::domain::Chat::new(Some(missing));
    chat.thread_id = Some("old-path".into());
    chat.draft = "Keep this draft".into();
    chat.set_workspace(Some(valid.clone()));
    assert_eq!(chat.workspace, Some(valid));
    assert!(chat.thread_id.is_none());
    assert_eq!(chat.previous_thread_ids, ["old-path"]);
    assert_eq!(chat.draft, "Keep this draft");
}

#[test]
fn malformed_input_is_rejected_and_errors_use_the_app_server_result_format() {
    for arguments in [
        json!({"action":"click","x":1,"y":2,"count":3}),
        json!({"action":"type","text":"x".repeat(8001)}),
        json!({"action":"scroll","x":0,"y":0,"clicks":i32::MIN}),
        json!({"action":"keypress","keys":[]}),
        json!({"action":"type","text":"ok","shell":"arbitrary"}),
        json!({"action":"click","x":1}),
    ] {
        assert!(Action::parse(&arguments).is_err());
    }
    assert!(Action::parse(&json!("{\"action\":\"screenshot\"}")).is_ok());
    let result = tool_result(Err("Window unavailable".into()));
    assert_eq!(result["success"], false);
    assert_eq!(result["contentItems"][0]["type"], "inputText");
    assert!(
        result["contentItems"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Window unavailable")
    );
}

#[tokio::test]
async fn disabled_cancelled_and_denied_actions_cannot_reach_the_desktop() {
    let (sender, mut events) = mpsc::channel(4);
    let (cancel, receiver) = watch::channel(false);
    let mut approved = false;
    assert!(
        execute(
            &json!({"action":"screenshot"}),
            &Settings::default(),
            &mut approved,
            &sender,
            receiver.clone()
        )
        .await
        .is_err()
    );
    assert!(events.try_recv().is_err());
    cancel.send(true).unwrap();
    let settings = Settings {
        enabled: true,
        confirm_actions: true,
    };
    assert!(
        execute(
            &json!({"action":"screenshot"}),
            &settings,
            &mut approved,
            &sender,
            receiver
        )
        .await
        .is_err()
    );
    assert!(events.try_recv().is_err());
    if !SUPPORTED {
        return;
    }
    let (_cancel, receiver) = watch::channel(false);
    let arguments = json!({"action":"type","text":"must not type"});
    let execution = execute(&arguments, &settings, &mut approved, &sender, receiver);
    let rejection = async {
        let ProviderEvent::Approval(approval) = events.recv().await.unwrap() else {
            panic!("Expected approval");
        };
        approval.response.send(false).unwrap();
    };
    let (result, ()) = tokio::join!(execution, rejection);
    assert!(result.unwrap_err().contains("not approved"));
    assert!(!approved);
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn approval_does_not_type_into_the_window_that_received_the_approval_click() {
    if !SUPPORTED {
        return;
    }
    let (sender, mut events) = mpsc::channel(4);
    let (_cancel, receiver) = watch::channel(false);
    let settings = Settings {
        enabled: true,
        confirm_actions: true,
    };
    let mut approved = false;
    let arguments = json!({"action":"type","text":"must not type"});
    let execution = execute(&arguments, &settings, &mut approved, &sender, receiver);
    let approval = async {
        let ProviderEvent::Approval(approval) = events.recv().await.unwrap() else {
            panic!("Expected approval");
        };
        approval.response.send(true).unwrap();
    };
    let (result, ()) = tokio::join!(execution, approval);
    assert!(approved);
    assert!(
        result.unwrap()[0]["text"]
            .as_str()
            .unwrap()
            .contains("No input was sent")
    );
    assert!(events.try_recv().is_err());
}

#[tokio::test]
#[ignore = "read-only installed Codex protocol check; no model generation"]
async fn installed_codex_accepts_native_desktop_tools() {
    let mut rpc = crate::codex::Rpc::connect(&crate::codex::find_executable())
        .await
        .unwrap();
    let result = rpc.call("thread/start",json!({"ephemeral":true,"sandbox":"read-only","approvalPolicy":"untrusted","dynamicTools":[tool_spec(),crate::delegation::tool_spec()]})).await;
    rpc.close().await;
    assert!(result.unwrap()["thread"]["id"].is_string());
}

#[cfg(windows)]
#[test]
#[ignore = "opens an isolated Windows test edit control, tests native input, captures screenshot in memory, then closes it; no model generation"]
fn native_desktop_round_trip_in_an_isolated_window() {
    use base64::Engine;
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    struct Window {
        handle: HWND,
        previous: HWND,
    }
    impl Drop for Window {
        fn drop(&mut self) {
            unsafe {
                DestroyWindow(self.handle);
                SetForegroundWindow(self.previous);
            }
        }
    }
    let class: Vec<u16> = "EDIT\0".encode_utf16().collect();
    let previous = unsafe { GetForegroundWindow() };
    let handle = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            null(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE | ES_MULTILINE as u32,
            80,
            80,
            600,
            300,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        )
    };
    assert!(!handle.is_null(), "Could not create test window");
    let window = Window { handle, previous };
    let (_cancel, receiver) = watch::channel(false);
    platform::execute(
        Action::Click {
            x: 140,
            y: 150,
            button: Button::Left,
            count: 1,
        },
        &receiver,
    )
    .unwrap();
    for _ in 0..20 {
        let mut message = MSG::default();
        while unsafe { PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) } != 0 {
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    platform::execute(
        Action::Focus {
            window: window.handle as usize as u64,
        },
        &receiver,
    )
    .unwrap();
    let titles = platform::execute(Action::Windows, &receiver).unwrap();
    assert_eq!(titles[0]["type"], "inputText");
    platform::execute(
        Action::Click {
            x: 140,
            y: 150,
            button: Button::Left,
            count: 1,
        },
        &receiver,
    )
    .unwrap();
    platform::execute(
        Action::Type {
            text: "Native desktop ✓".into(),
        },
        &receiver,
    )
    .unwrap();
    for _ in 0..20 {
        let mut message = MSG::default();
        while unsafe { PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) } != 0 {
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let mut text = [0u16; 128];
    let length = unsafe { GetWindowTextW(window.handle, text.as_mut_ptr(), text.len() as i32) };
    assert_eq!(
        String::from_utf16_lossy(&text[..length as usize]),
        "Native desktop ✓"
    );
    let screenshot = platform::execute(Action::Screenshot, &receiver).unwrap();
    assert_eq!(screenshot[1]["type"], "inputImage");
    let url = screenshot[1]["imageUrl"].as_str().unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(url.strip_prefix("data:image/jpeg;base64,").unwrap())
        .unwrap();
    let image = image::load_from_memory(&bytes).unwrap();
    assert!(image.width() > 0 && image.height() > 0);
    assert_eq!(platform::key_code("CTRL").unwrap(), 0x11);
    assert!(platform::key_code("not a key").is_err());
}
