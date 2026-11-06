use super::*;

#[test]
fn saved_cursor_choices_and_colors_survive_restart() {
    let config: Config = serde_json::from_str(
        r##"{
        "names":["Ada","Grace","Linus"],
        "name_choices":[false,true,true],
        "colors":["#123456","#ABCDEF"],
        "enabled":false
    }"##,
    )
    .unwrap();
    let startup = Startup::new(Some(config));
    let saved = serde_json::to_string(&startup.preferences()).unwrap();
    let restored = Startup::new(Some(serde_json::from_str(&saved).unwrap()));
    assert_eq!(restored.config.selected_names(), ["Grace", "Linus"]);
    assert_eq!(restored.config.colors, ["#123456", "#ABCDEF"]);
    assert!(!restored.is_active());
}

#[test]
fn old_configs_select_all_names_and_blank_names_preserve_selection_alignment() {
    let mut config: Config = serde_json::from_str(r#"{"names":["Ada","Grace"]}"#).unwrap();
    config.normalize();
    assert_eq!(config.selected_names(), ["Ada", "Grace"]);
    config.names = vec![" ".into(), " Grace ".into(), "Linus".into()];
    config.name_choices = vec![true, false, true];
    config.normalize();
    assert_eq!(config.names, ["Grace", "Linus"]);
    assert_eq!(config.selected_names(), ["Linus"]);
}

#[test]
fn empty_selections_and_palettes_recover_safely() {
    let mut config: Config = serde_json::from_str(
        r#"{
        "names":["Ada"],"name_choices":[false],"colors":["invalid"]
    }"#,
    )
    .unwrap();
    config.normalize();
    assert_eq!(config.selected_names(), ["Ada"]);
    assert!(!config.colors.is_empty());
    assert!(
        config
            .colors
            .iter()
            .all(|color| parse_color(color).is_some())
    );
}

#[test]
fn animation_covers_the_page_from_the_first_frame() {
    let mut startup = Startup {
        config: Config::default(),
        started: None,
        finished: false,
    };
    let context = egui::Context::default();
    let screen = Rect::from_min_size(Pos2::ZERO, vec2(1100.0, 900.0));
    let body = Rect::from_min_max(pos2(0.0, 42.0), screen.max);
    let page = Rect::from_min_size(pos2(200.0, 200.0), vec2(300.0, 200.0));
    for _ in 0..3 {
        let output = context.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |context| {
                context
                    .layer_painter(egui::LayerId::background())
                    .rect_filled(page, 0, Color32::RED);
                egui::TopBottomPanel::bottom("tall_composer")
                    .exact_height(861.0)
                    .show(context, |_| {});
                startup.draw(context);
            },
        );
        let page_index = output.shapes.iter().position(|entry| {
            matches!(&entry.shape, Shape::Rect(rect) if rect.rect == page && rect.fill == Color32::RED)
        }).expect("page was drawn for the animation to capture");
        assert!(
            output.shapes.iter().skip(page_index + 1).any(|entry| {
                matches!(&entry.shape, Shape::Rect(rect)
                if rect.rect.contains_rect(body)
                    && entry.clip_rect.contains_rect(body)
                    && rect.fill == super::super::theme::background()
                    && rect.fill.a() == 255)
            }),
            "animation must cover the page even during the area's first sizing pass"
        );
        assert!(startup.is_active());
    }
}

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
