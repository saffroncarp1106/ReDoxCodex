use super::*;

#[test]
fn live_reload_applies_saved_changes_and_preserves_last_valid_skin() {
    let path = std::env::temp_dir().join(format!("redox-live-{}.redoxskin", uuid::Uuid::new_v4()));
    let mut doc = Document::blank();
    doc.save(&path).unwrap();
    let mut skin = LiveSkin::load(&path);
    doc.name = "Live change".into();
    doc.save(&path).unwrap();
    skin.refresh(&path, true);
    assert_eq!(skin.document.as_ref().unwrap().name, "Live change");
    std::fs::write(&path, b"unfinished edit").unwrap();
    skin.refresh(&path, true);
    assert_eq!(skin.document.as_ref().unwrap().name, "Live change");
    assert!(skin.warning.is_some());
    doc.name = "Recovered design".into();
    doc.save(&path).unwrap();
    skin.refresh(&path, true);
    assert_eq!(skin.document.as_ref().unwrap().name, "Recovered design");
    assert!(skin.warning.is_none());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn authored_conversation_renders_active_chat_inside_its_panel_and_restores_theme() {
    use redox::domain::{Chat, Message, MessageStatus, Role};
    use redox_studio::document::{Kind, Node};
    let ctx = egui::Context::default();
    super::super::theme::apply(&ctx, "codex");
    let mut document = Document::blank();
    let mut panel = Node::new(1, Kind::Frame, 0., 0.);
    panel.width = 500.;
    panel.height = 230.;
    panel.padding = 18.;
    panel.fill = [252, 252, 252, 255];
    panel.text_color = [40, 50, 60, 255];
    panel.ui_script = vec![UiAction::Conversation];
    panel.text = "No messages yet".into();
    document.pages[0].nodes = vec![panel];
    let mut chat = Chat::new(None);
    let base = super::super::theme::palette();
    let run = |chat: &Chat| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(600., 600.),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    interactive::show_with_values(
                        ui,
                        &document,
                        Default::default(),
                        &mut |ui, control| {
                            assert_eq!(control.action, UiAction::Conversation);
                            conversation(ui, control, chat, true);
                        },
                    );
                });
            },
        )
    };
    let empty = run(&chat);
    assert!(empty.shapes.iter().any(
        |shape| matches!(&shape.shape, egui::Shape::Text(t) if t.galley.text() == "No messages yet")
    ));
    for (role, content) in [
        (Role::User, "My active prompt"),
        (Role::Assistant, "My active answer"),
    ] {
        chat.messages.push(Message {
            id: uuid::Uuid::new_v4(),
            role,
            content: content.into(),
            status: MessageStatus::Complete,
            error: None,
            model: String::new(),
            attachments: vec![],
            activity: Default::default(),
        });
    }
    run(&chat);
    let populated = run(&chat);
    for label in ["My active prompt", "My active answer"] {
        let shape = populated
            .shapes
            .iter()
            .find(|shape| matches!(&shape.shape, egui::Shape::Text(t) if t.galley.text() == label))
            .expect("Active chat is visible");
        assert!(shape.clip_rect.height() < 260.);
    }
    assert!(!populated.shapes.iter().any(
        |shape| matches!(&shape.shape, egui::Shape::Text(t) if t.galley.text() == "No messages yet")
    ));
    assert!(super::super::theme::palette() == base);
}

#[test]
fn local_skin_palette_is_restored_when_rendering_unwinds() {
    let base = super::super::theme::palette();
    let mut local = base;
    local.text = egui::Color32::RED;
    let result = std::panic::catch_unwind(|| {
        super::super::theme::with_palette(local, || {
            assert_eq!(super::super::theme::text(), egui::Color32::RED);
            panic!("render interrupted");
        })
    });
    assert!(result.is_err());
    assert!(super::super::theme::palette() == base);
}
