use super::*;

#[test]
fn model_list_stays_open_inside_a_centered_effort_panel() {
    let context = egui::Context::default();
    let options = vec![
        ("a".into(), "Model A".into()),
        ("b".into(), "Model B".into()),
    ];
    let mut selected = "a".to_string();
    let mut selector = egui::Rect::NOTHING;
    for frame in 0..7 {
        let events = if frame == 3 || frame == 4 {
            vec![
                egui::Event::PointerMoved(selector.center()),
                egui::Event::PointerButton {
                    pos: selector.center(),
                    button: egui::PointerButton::Primary,
                    pressed: frame == 3,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        } else {
            Vec::new()
        };
        let output = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(900.0, 700.0),
                )),
                events,
                ..Default::default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    ui.add_space(400.0);
                    ui.horizontal(|ui| {
                        ui.add_space(350.0);
                        let button = ui.button("Model and effort");
                        if frame == 0 {
                            egui::Popup::open_id(
                                context,
                                egui::Popup::default_response_id(&button),
                            );
                        }
                        let panel = egui::Popup::menu(&button)
                            .align(egui::emath::RectAlign::TOP)
                            .align_alternatives(&[egui::emath::RectAlign::BOTTOM])
                            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                            .show(|ui| {
                                ui.set_width(230.0);
                                model_choices(ui, "test_model", &mut selected, &options).rect
                            })
                            .expect("model selector must not dismiss the effort panel");
                        selector = panel.inner;
                        if frame >= 5 {
                            assert!(
                                (panel.response.rect.center().x - button.rect.center().x).abs()
                                    < 1.0
                            );
                        }
                    });
                });
            },
        );
        if frame >= 5 {
            assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text() == "Model B")),
                "model choices disappeared after opening");
        }
    }
}

#[test]
fn slider_selects_supported_steps_with_mouse_and_keyboard() {
    let context = egui::Context::default();
    let labels = vec!["Low".into(), "Medium".into(), "High".into()];
    let mut selected = 0;
    let mut rect = egui::Rect::NOTHING;
    let mut changed = false;
    for frame in 0..4 {
        let position = pos2(rect.right() - 13.0, rect.center().y);
        let events = match frame {
            1 | 2 => vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: frame == 1,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            3 => vec![egui::Event::Key {
                key: egui::Key::ArrowLeft,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            _ => Vec::new(),
        };
        let _ = context.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    let response = effort_steps(ui, &labels, &mut selected);
                    rect = response.rect;
                    changed |= response.changed();
                });
            },
        );
        if frame == 2 {
            assert_eq!(selected, 2);
        }
    }
    assert!(changed);
    assert_eq!(selected, 1);
}

#[test]
fn unsupported_or_single_effort_models_do_not_invent_levels() {
    let context = egui::Context::default();
    let _ = context.run(egui::RawInput::default(), |context| {
        egui::CentralPanel::default().show(context, |ui| {
            let mut selected = 0;
            assert!(!effort_steps(ui, &[], &mut selected).changed());
            let labels = vec!["Medium".into()];
            effort_steps(ui, &labels, &mut selected);
            assert_eq!(selected, 0);
        });
    });
}
