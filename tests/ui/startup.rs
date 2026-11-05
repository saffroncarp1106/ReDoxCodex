use super::*;

#[test]
fn actual_boxes_keep_their_geometry_and_dependencies() {
    let body = Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0));
    let panel = Rect::from_min_size(pos2(20.0, 50.0), vec2(500.0, 200.0));
    let button = Rect::from_min_size(pos2(40.0, 80.0), vec2(30.0, 30.0));
    let scene = Scene::from_shapes(
        vec![
            egui::epaint::ClippedShape {
                clip_rect: body,
                shape: Shape::rect_filled(panel, 20, Color32::GRAY),
            },
            egui::epaint::ClippedShape {
                clip_rect: body,
                shape: Shape::rect_filled(button, 8, Color32::BLUE),
            },
        ],
        body,
        12,
        42,
    );
    assert_eq!(scene.elements[0].bounds, panel);
    assert_eq!(scene.elements[0].rounding, egui::CornerRadius::same(20));
    assert!(scene.elements[1].start >= scene.elements[0].end);
    assert!(
        scene
            .elements
            .iter()
            .all(|element| element.fraction(scene.duration) == 1.0)
    );
}

#[test]
fn completed_animation_leaves_no_overlay() {
    let mut startup = Startup {
        config: Config::default(),
        started: Some(Instant::now() - Duration::from_secs(30)),
        finished: false,
    };
    let context = egui::Context::default();
    let output = context.run(egui::RawInput::default(), |context| startup.draw(context));
    assert!(!startup.is_active());
    assert!(output.shapes.is_empty());
}

#[test]
fn partial_configuration_has_safe_bounds() {
    let mut config: Config =
        serde_json::from_str(r#"{"cursor_count":999,"cursor_size":0}"#).unwrap();
    config.normalize();
    assert_eq!(config.cursor_count, 32);
    assert_eq!(config.cursor_size, 12.0);
}
