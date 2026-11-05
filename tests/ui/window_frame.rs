use super::*;

#[test]
fn unusable_saved_window_sizes_restore_to_default() {
    for size in [
        [237.0, 64.0],
        [1100.0, 64.0],
        [237.0, 900.0],
        [f32::NAN, 900.0],
    ] {
        let viewport =
            ForgeDesk::validate_window_size(egui::ViewportBuilder::default().with_inner_size(size));
        assert_eq!(viewport.inner_size, Some(egui::vec2(1100.0, 900.0)));
    }
}

#[test]
fn valid_saved_window_sizes_and_maximized_state_are_preserved() {
    let viewport = ForgeDesk::validate_window_size(
        egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 1000.0])
            .with_maximized(true),
    );
    assert_eq!(viewport.inner_size, Some(egui::vec2(1400.0, 1000.0)));
    assert_eq!(viewport.maximized, Some(true));
}
