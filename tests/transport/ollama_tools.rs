use super::*;

fn project() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("temp/verification/ollama-tools")
        .join(uuid::Uuid::new_v4().to_string())
        .join("project");
    std::fs::create_dir_all(&root).unwrap();
    root
}

#[tokio::test]
async fn project_reads_work_but_traversal_and_read_only_writes_are_rejected() {
    let root = project();
    std::fs::write(root.join("note.txt"), "LOCAL_PROJECT_DATA").unwrap();
    let (sender, mut events) = mpsc::channel(8);
    assert_eq!(
        execute(
            "read_file",
            &json!({"path":"note.txt"}),
            Some(&root),
            Mode::Chat,
            &sender
        )
        .await
        .unwrap(),
        "LOCAL_PROJECT_DATA"
    );
    assert!(
        execute(
            "list_files",
            &json!({"path":"."}),
            Some(&root),
            Mode::Chat,
            &sender
        )
        .await
        .unwrap()
        .contains("note.txt")
    );
    assert!(
        execute(
            "read_file",
            &json!({"path":"../outside.txt"}),
            Some(&root),
            Mode::Code,
            &sender
        )
        .await
        .is_err()
    );
    assert!(
        execute(
            "write_file",
            &json!({"path":"bad.txt","content":"bad"}),
            Some(&root),
            Mode::Chat,
            &sender
        )
        .await
        .is_err()
    );
    assert!(
        execute(
            "run_command",
            &json!({"command":"echo forbidden"}),
            Some(&root),
            Mode::Chat,
            &sender
        )
        .await
        .is_err()
    );
    assert!(!root.join("bad.txt").exists());
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn workspace_writes_wait_for_approval_and_rejection_keeps_files_unchanged() {
    let root = project();
    let (sender, mut events) = mpsc::channel(8);
    let reject = async {
        if let Some(ProviderEvent::Approval(approval)) = events.recv().await {
            assert!(approval.description.contains("new.txt"));
            assert!(!root.join("new.txt").exists());
            approval.response.send(false).unwrap();
        } else {
            panic!("No approval request");
        }
    };
    let args = json!({"path":"new.txt","content":"new content"});
    let (result, _) = tokio::join!(
        execute("write_file", &args, Some(&root), Mode::Code, &sender),
        reject
    );
    assert!(result.is_err());
    assert!(!root.join("new.txt").exists());
    let accept = async {
        if let Some(ProviderEvent::Approval(approval)) = events.recv().await {
            approval.response.send(true).unwrap();
        } else {
            panic!("No approval request");
        }
    };
    let (result, _) = tokio::join!(
        execute("write_file", &args, Some(&root), Mode::Code, &sender),
        accept
    );
    assert!(result.is_ok());
    assert_eq!(
        std::fs::read_to_string(root.join("new.txt")).unwrap(),
        "new content"
    );
}

#[test]
fn absolute_outside_paths_and_symlink_escapes_are_confined_to_full_access() {
    let root = project();
    let outside = root.parent().unwrap().join("outside.txt");
    std::fs::write(&outside, "private").unwrap();
    assert!(resolve_path(&root, outside.to_str().unwrap(), Mode::Code, false).is_err());
    assert_eq!(
        resolve_path(&root, outside.to_str().unwrap(), Mode::FullAccess, false).unwrap(),
        outside.canonicalize().unwrap()
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&outside, root.join("link.txt")).unwrap();
        assert!(resolve_path(&root, "link.txt", Mode::Code, true).is_err());
        std::os::unix::fs::symlink(
            root.parent().unwrap().join("missing-outside.txt"),
            root.join("dangling.txt"),
        )
        .unwrap();
        assert!(resolve_path(&root, "dangling.txt", Mode::Code, true).is_err());
    }
}

#[tokio::test]
async fn commands_are_real_and_read_only_tool_definitions_exclude_mutations() {
    let root = project();
    assert!(
        run_command("echo LOCAL_COMMAND_OK", &root)
            .await
            .unwrap()
            .contains("LOCAL_COMMAND_OK")
    );
    let read_only = definitions(Mode::Chat).to_string();
    assert!(read_only.contains("read_file"));
    assert!(!read_only.contains("write_file"));
    assert!(!read_only.contains("run_command"));
}
