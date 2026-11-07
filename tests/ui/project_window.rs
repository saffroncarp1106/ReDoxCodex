use super::*;
use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Sense, pos2, vec2};

#[test]
fn only_the_inside_border_activates_the_glow() {
    let rect = Rect::from_min_size(pos2(10.0, 20.0), vec2(180.0, 30.0));
    assert!(border_contains(rect, pos2(11.0, 35.0)));
    assert!(border_contains(rect, pos2(100.0, 21.0)));
    assert!(!border_contains(rect, rect.center()));
    assert!(!border_contains(rect, pos2(9.0, 35.0)));
}

#[test]
fn clicking_or_dragging_inside_does_not_detach_but_releasing_outside_does() {
    let ctx = egui::Context::default();
    let run = |events| {
        let mut position = None;
        let _ = ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0))),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let (_, response) =
                        ui.allocate_exact_size(vec2(200.0, 30.0), Sense::click_and_drag());
                    position = detach_position(&response);
                });
            },
        );
        position
    };
    let button = |pos, pressed| Event::PointerButton {
        pos,
        pressed,
        button: PointerButton::Primary,
        modifiers: Modifiers::NONE,
    };
    let start = pos2(60.0, 20.0);
    let inside = pos2(150.0, 20.0);
    let outside = pos2(360.0, 20.0);
    assert_eq!(run(vec![Event::PointerMoved(start)]), None);
    assert_eq!(run(vec![button(start, true)]), None);
    assert_eq!(run(vec![button(start, false)]), None);
    assert_eq!(run(vec![button(start, true)]), None);
    assert_eq!(run(vec![Event::PointerMoved(inside)]), None);
    assert_eq!(run(vec![button(inside, false)]), None);
    assert_eq!(
        run(vec![Event::PointerMoved(start), button(start, true)]),
        None
    );
    assert_eq!(run(vec![Event::PointerMoved(outside)]), None);
    assert_eq!(run(vec![button(outside, false)]), Some(outside));
}
