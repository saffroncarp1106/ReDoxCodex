use super::*;
use redox::domain::Chat;

#[test]
fn switching_projects_and_sidebar_width_keeps_text_inside_the_transcript_column() {
    let context = egui::Context::default();
    let mut chat = Chat::new(Some("project".into()));
    for (role, content) in [
        (Role::User, format!("```text\n{}\n```", "x".repeat(500))),
        (
            Role::Assistant,
            "Assistant response stays inside the chat pane.".into(),
        ),
    ] {
        chat.messages.push(Message {
            id: uuid::Uuid::new_v4(),
            role,
            content,
            status: MessageStatus::Complete,
            error: None,
            model: String::new(),
            attachments: Vec::new(),
            activity: Default::default(),
        });
    }
    let other_project = uuid::Uuid::new_v4();
    for (screen_width, sidebar_width, project) in [
        (1_600.0, 60.0, chat.id),
        (1_600.0, 350.0, other_project),
        (640.0, 350.0, chat.id),
    ] {
        let mut expected_left = 0.0;
        let output = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(screen_width, 700.0),
                )),
                ..Default::default()
            },
            |context| {
                egui::SidePanel::left("projects")
                    .exact_width(sidebar_width)
                    .show(context, |_| {});
                egui::CentralPanel::default().show(context, |ui| {
                    let width = (ui.available_width() - 48.0)
                        .clamp(260.0, 820.0)
                        .min(ui.available_width());
                    expected_left = ui.available_rect_before_wrap().center().x - width * 0.5;
                    render_transcript(ui, &chat, ("test_project", project), 48.0, false);
                });
            },
        );
        let x = output
            .shapes
            .iter()
            .find_map(|entry| {
                let egui::Shape::Text(text) = &entry.shape else {
                    return None;
                };
                text.galley
                    .text()
                    .starts_with("Assistant")
                    .then_some(text.pos.x)
            })
            .expect("assistant response is painted");
        assert!(
            (x - expected_left).abs() < 2.0,
            "text starts at {x}, expected {expected_left}"
        );
    }
}
