use redox::activity::{ActivityUpdate, CommandActivity};
use redox::domain::Session;
use redox::run::{RunEvent, Update, apply_event};
use serde_json::json;

#[test]
fn turn_activity_updates_the_inline_command_and_replaces_the_reported_diff() {
    let mut session = Session::default();
    session.new_chat();
    session.active_mut().unwrap().draft = "Update the project".into();
    let run = session.prepare_run().unwrap();
    let mut emit = |update| {
        apply_event(&mut session, Some(run), RunEvent { run, update });
    };
    let command = |status, exit_code| Update::Activity {
        turn_id: "turn-1".into(),
        update: ActivityUpdate::Command(
            CommandActivity::from_item(&json!({
                "id":"command-1", "type":"commandExecution", "command":"cargo test",
                "status":status, "exitCode":exit_code
            }))
            .unwrap(),
        ),
    };
    let diff = |patch: &str| Update::Activity {
        turn_id: "turn-1".into(),
        update: ActivityUpdate::Diff(patch.into()),
    };
    let intro = "I’ll test the changes.\n\n";
    emit(Update::Content(intro.into()));
    emit(command("inProgress", None));
    emit(Update::Content("Tests passed.".into()));
    emit(command("completed", Some(0)));
    emit(diff(
        "diff --git a/old.txt b/old.txt\n--- a/old.txt\n+++ b/old.txt\n@@ -1 +1 @@\n-old\n+new\n",
    ));
    emit(diff(concat!(
        "diff --git a/src/file name.rs b/src/file name.rs\n",
        "--- a/src/file name.rs\n+++ b/src/file name.rs\n@@ -1 +1 @@\n-old\n+++new\n",
        "diff --git a/deleted.txt b/deleted.txt\n",
        "--- a/deleted.txt\n+++ /dev/null\n@@ -1 +0,0 @@\n-gone\n",
    )));
    emit(Update::Complete);

    let response = session.active().unwrap().messages.last().unwrap();
    assert_eq!(response.content, format!("{intro}Tests passed."));
    assert_eq!(response.activity.commands.len(), 1);
    assert_eq!(response.activity.commands[0].offset, intro.len());
    assert_eq!(response.activity.commands[0].exit_code, Some(0));
    let files = &response.activity.files;
    assert_eq!(files.len(), 2);
    assert_eq!(
        (files[0].path.as_str(), files[0].added, files[0].removed),
        ("src/file name.rs", 1, 1)
    );
    assert_eq!(
        (files[1].path.as_str(), files[1].added, files[1].removed),
        ("deleted.txt", 0, 1)
    );
}
