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

#[test]
fn long_archived_project_names_do_not_expand_the_sidebar() {
    let context = egui::Context::default();
    win98::apply_style(&context);
    let long_name = "archived-project-".repeat(40);
    let path = std::path::Path::new(&long_name);
    let _ = context.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(900.0, 700.0),
            )),
            ..Default::default()
        },
        |context| {
            egui::SidePanel::left("archive_width_test")
                .exact_width(260.0)
                .show(context, |ui| {
                    let available = ui.available_width();
                    let row = ui.scope(|ui| archived_project_row(ui, path));
                    assert!(
                        row.response.rect.height() <= 26.0,
                        "archived row expanded vertically: {}",
                        row.response.rect.height()
                    );
                    assert!(
                        row.response.rect.width() <= available + 0.5,
                        "archived row widened the sidebar: {} > {available}",
                        row.response.rect.width()
                    );
                });
        },
    );
}

#[test]
fn expanded_archives_are_capped_even_with_many_projects() {
    let context = egui::Context::default();
    win98::apply_style(&context);
    let paths: Vec<_> = (0..100)
        .map(|i| std::path::PathBuf::from(format!("Archived project {i}")))
        .collect();
    let _ = context.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(900.0, 700.0),
            )),
            ..Default::default()
        },
        |context| {
            egui::SidePanel::left("archive_height_test")
                .exact_width(260.0)
                .show(context, |ui| {
                    let body = ui.scope(|ui| archived_projects_body(ui, &paths));
                    assert!(
                        body.response.rect.height() <= 121.0,
                        "archive list exceeded its height limit: {}",
                        body.response.rect.height()
                    );
                    assert!(
                        ui.available_height() > 500.0,
                        "archives consumed the sidebar"
                    );
                });
        },
    );
}
