use super::*;

#[test]
fn fifth_logo_click_unlocks_win98_and_survives_restart() {
    let mut preferences = Preferences::default();
    let mut about = About::default();
    assert!(!preferences.shows_ui(win98::ID));
    assert!(preferences.shows_ui("codex"));
    for _ in 0..4 {
        about.click_logo(&mut preferences);
        assert!(!preferences.shows_ui(win98::ID));
        assert!(!about.show_notice);
    }
    about.click_logo(&mut preferences);
    assert!(preferences.shows_ui(win98::ID));
    assert!(about.show_notice);
    assert_eq!(preferences.ui, "codex");

    let json = serde_json::to_string(&preferences).unwrap();
    let mut restarted: Preferences = serde_json::from_str(&json).unwrap();
    restarted.normalize();
    let mut about = About::default();
    about.click_logo(&mut restarted);
    assert!(restarted.shows_ui(win98::ID));
    assert!(!about.show_notice);
}

#[test]
fn previously_selected_win98_stays_available() {
    for saved in [r#"{"ui":"win98"}"#, r#"{"sidebars":{"win98":false}}"#] {
        let mut preferences: Preferences = serde_json::from_str(saved).unwrap();
        preferences.normalize();
        assert!(preferences.shows_ui(win98::ID));
    }
}

#[test]
fn extra_clicks_outside_the_unlock_dialog_do_not_dismiss_it() {
    let context = egui::Context::default();
    theme::apply(&context, "codex");
    let position = egui::pos2(30.0, 30.0);
    let frame = |events| {
        let mut closed = false;
        let mut clicked_behind = false;
        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(800.0, 600.0),
                )),
                events,
                ..Default::default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    clicked_behind = ui
                        .add_sized([120.0, 50.0], egui::Button::new("ReDox logo"))
                        .clicked();
                });
                closed = unlock_notice(context);
            },
        );
        (closed, clicked_behind)
    };
    frame(Vec::new());
    frame(Vec::new());
    for _ in 0..2 {
        for pressed in [true, false] {
            assert_eq!(
                frame(vec![egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                }]),
                (false, false)
            );
        }
    }
    assert_eq!(
        frame(vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]),
        (true, false)
    );
}
