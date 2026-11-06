use super::*;

fn input(events: Vec<egui::Event>, command: bool) -> egui::RawInput {
    egui::RawInput {
        events,
        modifiers: egui::Modifiers {
            command,
            ctrl: command,
            ..Default::default()
        },
        focused: true,
        ..Default::default()
    }
}

fn v(pressed: bool, command: bool) -> egui::Event {
    egui::Event::Key {
        key: egui::Key::V,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers {
            command,
            ctrl: command,
            ..Default::default()
        },
    }
}

#[test]
fn paste_events_trigger_once_and_regular_typing_does_not_attach_files() {
    let mut gate = PasteGate::default();
    assert!(gate.requested(&input(vec![egui::Event::Paste("text".into())], true)));
    assert!(!gate.requested(&input(vec![v(false, true)], true)));
    assert!(!gate.requested(&input(vec![v(true, false)], false)));
    assert!(!gate.requested(&input(vec![v(false, false)], false)));
}

#[test]
fn file_paste_works_when_egui_consumes_the_press_and_control_is_released_first() {
    let mut gate = PasteGate::default();
    assert!(!gate.requested(&input(Vec::new(), true)));
    assert!(!gate.requested(&input(Vec::new(), false)));
    assert!(gate.requested(&input(vec![v(false, false)], false)));
    assert!(!gate.requested(&input(vec![v(false, false)], false)));
}

#[test]
fn old_shortcuts_do_not_turn_later_plain_v_typing_into_paste() {
    let mut gate = PasteGate::default();
    assert!(!gate.requested(&input(Vec::new(), true)));
    assert!(!gate.requested(&input(vec![v(true, false)], false)));
    assert!(!gate.requested(&input(vec![v(false, false)], false)));
}
