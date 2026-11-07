use super::*;
use redox::domain::{MessageStatus, Role};
use uuid::Uuid;

fn assistant(content: &str) -> Message {
    Message {
        id: Uuid::new_v4(),
        role: Role::Assistant,
        content: content.into(),
        status: MessageStatus::Complete,
        error: None,
        model: String::new(),
        attachments: Vec::new(),
    }
}

fn draw(context: &egui::Context, message: &Message, input: egui::RawInput) -> egui::FullOutput {
    context.run(input, |context| {
        egui::CentralPanel::default().show(context, |ui| show(ui, message, None));
    })
}

#[test]
fn number_animates_on_appearance_and_replays_after_leaving_view() {
    let context = egui::Context::default();
    let message = assistant("Numbers: 167 and **67**.");
    let input = |time| egui::RawInput {
        time: Some(time),
        ..Default::default()
    };
    let highlighted = |output: egui::FullOutput| {
        output.shapes.iter().filter(|entry| {
            matches!(&entry.shape, Shape::Text(text) if text.galley.rows.iter().any(|row| {
                row.visuals.mesh.vertices.iter().any(|vertex| vertex.color == super::super::theme::blue())
            }))
        }).count()
    };
    let number_vertices = |output: &egui::FullOutput| {
        output
            .shapes
            .iter()
            .find_map(|entry| {
                let Shape::Text(text) = &entry.shape else {
                    return None;
                };
                (text.galley.text() == "67")
                    .then(|| text.galley.rows[0].visuals.mesh.vertices.clone())
            })
            .unwrap()
    };
    assert_eq!(highlighted(draw(&context, &message, input(0.0))), 1);
    let animated = draw(&context, &message, input(1.0 / 6.0));
    let rotated = number_vertices(&animated);
    assert_eq!(highlighted(animated), 1);

    let expired = draw(&context, &message, input(2.1));
    let upright = number_vertices(&expired);
    assert!(
        rotated
            .iter()
            .zip(upright)
            .any(|(animated, normal)| animated.pos.distance(normal.pos) > 0.5)
    );
    assert_eq!(highlighted(expired), 0);
    assert_eq!(highlighted(draw(&context, &message, input(2.2))), 0);

    let hidden = context.run(input(2.3), |context| {
        egui::CentralPanel::default().show(context, |ui| {
            ui.set_clip_rect(egui::Rect::from_min_max(
                egui::pos2(0.0, 200.0),
                egui::pos2(800.0, 600.0),
            ));
            show(ui, &message, None);
        });
    });
    assert_eq!(highlighted(hidden), 0);
    assert_eq!(highlighted(draw(&context, &message, input(2.4))), 1);
}

#[test]
fn tooltip_is_attached_to_the_number() {
    let context = egui::Context::default();
    context.style_mut(|style| {
        style.interaction.tooltip_delay = 0.0;
        style.interaction.show_tooltips_only_when_still = false;
    });
    let message = assistant("Look at 727.");
    let output = draw(&context, &message, Default::default());
    let hit = output
        .shapes
        .iter()
        .find_map(|entry| {
            let Shape::Text(text) = &entry.shape else {
                return None;
            };
            let bytes = number_ranges(text.galley.text(), "727").next()?;
            glyph_rects(text, bytes).next()
        })
        .unwrap()
        .center();
    let input = egui::RawInput {
        events: vec![egui::Event::PointerMoved(hit)],
        ..Default::default()
    };
    let output = draw(&context, &message, input);
    let tooltip = |output: &egui::FullOutput| {
        output
            .shapes
            .iter()
            .any(|entry| matches!(&entry.shape, Shape::Text(text) if text.galley.text() == "WYSI"))
    };
    assert!(tooltip(&output) || tooltip(&draw(&context, &message, Default::default())));
    let input = egui::RawInput {
        events: vec![egui::Event::PointerMoved(hit + egui::vec2(100.0, 0.0))],
        ..Default::default()
    };
    assert!(!tooltip(&draw(&context, &message, input)));
    let input = egui::RawInput {
        events: vec![egui::Event::PointerMoved(hit)],
        ..Default::default()
    };
    let output = draw(&context, &message, input);
    assert!(tooltip(&output) || tooltip(&draw(&context, &message, Default::default())));
}
