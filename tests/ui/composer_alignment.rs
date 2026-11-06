use super::*;

#[test]
fn controls_stay_centered_after_resizing_and_label_changes() {
    let context = egui::Context::default();
    for (width, model) in [
        (900.0, "GPT 6.1 Sol"),
        (1200.0, "A longer local model"),
        (600.0, "Qwen"),
    ] {
        for frame in 0..3 {
            let _ = context.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(width, 500.0))),
                    ..Default::default()
                },
                |context| {
                    egui::CentralPanel::default().show(context, |ui| {
                        let center = ui.available_rect_before_wrap().center().x;
                        let mut controls = Rect::NOTHING;
                        centered_controls(ui, "alignment", |ui| {
                            controls = ui.button(model).rect;
                            controls = controls.union(ui.button("Medium").rect);
                            controls = controls.union(ui.button("Workspace access").rect);
                        });
                        if frame > 0 || context.current_pass_index() > 0 {
                            assert!(
                                (controls.center().x - center).abs() < 1.0,
                                "controls center {} differs from input center {center}",
                                controls.center().x
                            );
                        }
                    });
                },
            );
        }
    }
}
