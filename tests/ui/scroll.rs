use super::*;

#[test]
fn classic_thumb_stays_between_the_arrow_buttons_on_both_axes() {
    for axis in [0, 1] {
        let size = if axis == 1 {
            vec2(17.0, 220.0)
        } else {
            vec2(220.0, 17.0)
        };
        let bar = Rect::from_min_size(pos2(10.0, 20.0), size);
        let (first, track, last, top) = geometry(bar, axis, 200.0, 1000.0, 0.0);
        let (_, _, _, bottom) = geometry(bar, axis, 200.0, 1000.0, 800.0);
        assert_eq!(first.size()[axis], WIDTH);
        assert_eq!(last.size()[axis], WIDTH);
        assert_eq!(top.min[axis], track.min[axis]);
        assert!((bottom.max[axis] - track.max[axis]).abs() < 0.01);
        assert!(track.contains_rect(top) && track.contains_rect(bottom));
    }
}

fn frame(context: &egui::Context, events: Vec<egui::Event>, sticky: bool) -> (Rect, f32) {
    let mut result = (Rect::NOTHING, 0.0);
    let _ = context.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(320.0, 240.0))),
            events,
            ..Default::default()
        },
        |context| {
            egui::CentralPanel::default().show(context, |ui| {
                let output = ScrollArea::vertical()
                    .id_salt("test_scroll")
                    .max_height(180.0)
                    .auto_shrink([false, false])
                    .stick_to_bottom(sticky)
                    .show(ui, |ui| {
                        ui.allocate_space(vec2(80.0, 900.0));
                    });
                result = (output.inner_rect, output.state.offset.y);
            });
        },
    );
    result
}

fn click(context: &egui::Context, point: egui::Pos2, sticky: bool) {
    for pressed in [true, false] {
        frame(
            context,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            sticky,
        );
    }
}

#[test]
fn arrow_click_scrolls_and_does_not_snap_back_to_the_bottom() {
    let context = egui::Context::default();
    super::super::win98::apply_style(&context);
    let (rect, before) = frame(&context, vec![], true);
    frame(&context, vec![], true);
    click(
        &context,
        pos2(rect.right() + WIDTH / 2.0, rect.top() + WIDTH / 2.0),
        true,
    );
    let (_, after) = frame(&context, vec![], true);
    assert!(
        after < before - 1.0,
        "arrow did not move upward: {before} -> {after}"
    );
    let (_, stable) = frame(&context, vec![], true);
    assert!((stable - after).abs() < 0.01, "scroll snapped back");
}

#[test]
fn down_arrow_moves_content_without_resizing_the_viewport() {
    let context = egui::Context::default();
    super::super::win98::apply_style(&context);
    let (rect, _) = frame(&context, vec![], false);
    frame(&context, vec![], false);
    click(
        &context,
        pos2(rect.right() + WIDTH / 2.0, rect.bottom() - WIDTH / 2.0),
        false,
    );
    let (after_rect, offset) = frame(&context, vec![], false);
    assert!(offset >= 24.0);
    assert_eq!(rect, after_rect);
}

#[test]
fn dragging_the_visible_thumb_moves_the_native_scroll_state() {
    let context = egui::Context::default();
    super::super::win98::apply_style(&context);
    let (rect, _) = frame(&context, vec![], false);
    frame(&context, vec![], false);
    let start = pos2(rect.right() + WIDTH / 2.0, rect.top() + WIDTH + 10.0);
    frame(
        &context,
        vec![
            egui::Event::PointerMoved(start),
            egui::Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        false,
    );
    frame(
        &context,
        vec![egui::Event::PointerMoved(start + vec2(0.0, 50.0))],
        false,
    );
    let (_, offset) = frame(
        &context,
        vec![egui::Event::PointerButton {
            pos: start + vec2(0.0, 50.0),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
        false,
    );
    assert!(
        offset > 100.0,
        "visible thumb drag did not scroll: {offset}"
    );
}
