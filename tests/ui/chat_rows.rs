use super::*;

#[test]
fn archive_click_does_not_select_the_chat() {
    let context = egui::Context::default();
    let mut button = egui::Rect::NOTHING;
    let mut row_clicked = false;
    let mut archive_clicked = false;
    for frame in 0..3 {
        let events = if frame == 0 {
            Vec::new()
        } else {
            vec![
                egui::Event::PointerMoved(button.center()),
                egui::Event::PointerButton {
                    pos: button.center(),
                    button: egui::PointerButton::Primary,
                    pressed: frame == 1,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        let _ = context.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    let (row, archive) =
                        chat_row(ui, "A long conversation title", false, 8.0, false, true);
                    assert!(row.rect.right() <= archive.rect.left());
                    button = archive.rect;
                    row_clicked |= row.clicked();
                    archive_clicked |= archive.clicked();
                });
            },
        );
    }
    assert!(archive_clicked);
    assert!(!row_clicked);
}

#[test]
fn prompt_focus_waits_for_dialogs_and_is_only_requested_once() {
    let context = egui::Context::default();
    let mut pending = true;
    let mut text = String::new();
    for frame in 0..3 {
        let _ = context.run(egui::RawInput::default(), |context| {
            egui::CentralPanel::default().show(context, |ui| {
                let prompt = ui.text_edit_singleline(&mut text);
                let other = ui.button("Another control");
                if frame == 2 {
                    other.request_focus();
                }
                super::super::controls::focus_new_prompt(&prompt, &mut pending, frame > 0);
                if frame == 0 {
                    assert!(pending);
                }
                if frame == 1 {
                    assert!(prompt.has_focus());
                }
                if frame == 2 {
                    assert!(other.has_focus());
                }
            });
        });
    }
    assert!(!pending);
}
