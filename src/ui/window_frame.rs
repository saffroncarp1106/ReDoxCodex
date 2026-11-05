use eframe::egui::{self, CursorIcon, Pos2, Rect, ResizeDirection};

use crate::app::ForgeDesk;

pub(super) fn title_drag(context: &egui::Context, response: &egui::Response) {
    if response.drag_started() {
        context.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if response.double_clicked() {
        let maximized = context.input(|input| input.viewport().maximized.unwrap_or(false));
        context.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }
}

impl ForgeDesk {
    pub(in crate::app) fn window_interactions(&mut self, context: &egui::Context) {
        if context.input(|input| input.viewport().maximized.unwrap_or(false)) {
            return;
        }
        let Some(pointer) = context.input(|input| input.pointer.hover_pos()) else {
            return;
        };
        if let Some((direction, cursor)) = resize_hit(context.viewport_rect(), pointer) {
            context.set_cursor_icon(cursor);
            if context.input(|input| input.pointer.primary_pressed()) {
                context.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
            }
        }
    }
}

fn resize_hit(rect: Rect, pointer: Pos2) -> Option<(ResizeDirection, CursorIcon)> {
    if !rect.contains(pointer) {
        return None;
    }
    let left = pointer.x - rect.left() <= 6.0;
    let right = rect.right() - pointer.x <= 6.0;
    let top = pointer.y - rect.top() <= 6.0;
    let bottom = rect.bottom() - pointer.y <= 6.0;
    match (left, right, top, bottom) {
        (true, _, true, _) => Some((ResizeDirection::NorthWest, CursorIcon::ResizeNwSe)),
        (_, true, true, _) => Some((ResizeDirection::NorthEast, CursorIcon::ResizeNeSw)),
        (true, _, _, true) => Some((ResizeDirection::SouthWest, CursorIcon::ResizeNeSw)),
        (_, true, _, true) => Some((ResizeDirection::SouthEast, CursorIcon::ResizeNwSe)),
        (true, _, _, _) => Some((ResizeDirection::West, CursorIcon::ResizeHorizontal)),
        (_, true, _, _) => Some((ResizeDirection::East, CursorIcon::ResizeHorizontal)),
        (_, _, true, _) => Some((ResizeDirection::North, CursorIcon::ResizeVertical)),
        (_, _, _, true) => Some((ResizeDirection::South, CursorIcon::ResizeVertical)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{pos2, vec2};

    #[test]
    fn edges_and_corners_match_the_native_resize_direction() {
        let rect = Rect::from_min_size(pos2(20.0, 30.0), vec2(1000.0, 800.0));
        assert!(matches!(
            resize_hit(rect, pos2(21.0, 31.0)),
            Some((ResizeDirection::NorthWest, _))
        ));
        assert!(matches!(
            resize_hit(rect, pos2(1019.0, 829.0)),
            Some((ResizeDirection::SouthEast, _))
        ));
        assert!(matches!(
            resize_hit(rect, pos2(1019.0, 400.0)),
            Some((ResizeDirection::East, _))
        ));
        assert!(matches!(
            resize_hit(rect, pos2(500.0, 829.0)),
            Some((ResizeDirection::South, _))
        ));
    }

    #[test]
    fn content_and_points_outside_the_window_are_not_resize_targets() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 800.0));
        assert!(resize_hit(rect, pos2(100.0, 100.0)).is_none());
        assert!(resize_hit(rect, pos2(-1.0, 0.0)).is_none());
    }

    #[test]
    fn dragging_the_title_area_emits_a_native_move_request() {
        let context = egui::Context::default();
        let draw = |context: &egui::Context| {
            egui::TopBottomPanel::top("title_test")
                .exact_height(42.0)
                .show(context, |ui| {
                    let (_, response) =
                        ui.allocate_exact_size(vec2(600.0, 30.0), egui::Sense::click_and_drag());
                    title_drag(context, &response);
                });
        };
        let press = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 800.0))),
            events: vec![
                egui::Event::PointerMoved(pos2(200.0, 20.0)),
                egui::Event::PointerButton {
                    pos: pos2(200.0, 20.0),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::default(),
                },
            ],
            ..Default::default()
        };
        let warmup = egui::RawInput {
            screen_rect: press.screen_rect,
            ..Default::default()
        };
        let _ = context.run(warmup, draw);
        let press_output = context.run(press, draw);
        let motion = egui::RawInput {
            events: vec![egui::Event::PointerMoved(pos2(320.0, 20.0))],
            ..Default::default()
        };
        let output = context.run(motion, draw);
        assert!(
            press_output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .iter()
                .chain(
                    output.viewport_output[&egui::ViewportId::ROOT]
                        .commands
                        .iter()
                )
                .any(|command| matches!(command, egui::ViewportCommand::StartDrag))
        );
    }
}
