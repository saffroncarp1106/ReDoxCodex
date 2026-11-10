use super::*;

#[test]
fn light_controls_keep_wii_rounding_and_do_not_use_classic_scrollbars() {
	let context = egui::Context::default();
	theme::apply(&context, ID);
	let _ = context.run(egui::RawInput::default(), |context| {
		egui::CentralPanel::default().show(context, |ui| {
			assert!(!ui.visuals().dark_mode);
			assert!(!theme::is_classic(ui));
			assert!(theme::is_wii(context));
			assert_eq!(ui.spacing().scroll.bar_width, 8.0);
			assert_eq!(ui.visuals().widgets.hovered.bg_stroke.color, BLUE);
		});
	});
}

#[test]
fn exported_player_sprites_rasterize_with_transparency_and_distinct_tints() {
	let mut images = Vec::new();
	for sprite in [
		include_bytes!("../../src/ui/assets/wii/cursor-blue.svg").as_slice(),
		include_bytes!("../../src/ui/assets/wii/cursor-red.svg").as_slice(),
		include_bytes!("../../src/ui/assets/wii/cursor-green.svg").as_slice(),
		include_bytes!("../../src/ui/assets/wii/cursor-gold.svg").as_slice(),
	] {
		let image = egui_extras::image::load_svg_bytes(sprite, &Default::default()).unwrap();
		assert_eq!(image.size, [85, 126]);
		assert!(image.pixels.iter().any(|pixel| pixel.a() == 0));
		assert!(image.pixels.iter().any(|pixel| pixel.a() == 255));
		images.push(image.pixels);
	}
	for index in 1..images.len() {
		assert_ne!(images[0], images[index]);
	}
}

#[test]
fn wii_controls_support_keyboard_activation_and_ignore_disabled_clicks() {
	let context = egui::Context::default();
	theme::apply(&context, ID);
	let mut activated = false;
	for frame in 0..2 {
		let events = if frame == 1 {
			vec![egui::Event::Key {
				key: egui::Key::Enter,
				physical_key: None,
				pressed: true,
				repeat: false,
				modifiers: egui::Modifiers::NONE,
			}]
		}
		else {
			Vec::new()
		};
		let _ = context.run(egui::RawInput { events, ..Default::default() }, |context| {
			egui::CentralPanel::default().show(context, |ui| {
				let response = button(ui, "New chat", 110.0, false);
				if frame == 0 {
					response.request_focus();
				}
				activated |= response.clicked();
				let disabled = ui.add_enabled_ui(false, |ui| button(ui, "Unavailable", 110.0, false)).inner;
				assert!(!disabled.enabled());
				assert!(!disabled.clicked());
			});
		});
	}
	assert!(activated);
}

#[test]
fn wii_preferences_round_trip_without_affecting_other_layouts() {
	let mut prefs = super::super::Preferences::default();
	prefs.wii.sounds = false;
	prefs.wii.motion = false;
	let restored: super::super::Preferences = serde_json::from_str(&serde_json::to_string(&prefs).unwrap()).unwrap();
	assert!(!restored.wii.sounds);
	assert!(!restored.wii.motion);
	assert_eq!(restored.ui, "codex");
}

#[test]
fn blue_pointer_preserves_text_and_resize_cursors() {
	let context = egui::Context::default();
	theme::apply(&context, ID);
	for cursor in [egui::CursorIcon::Default, egui::CursorIcon::PointingHand, egui::CursorIcon::Text, egui::CursorIcon::ResizeHorizontal] {
		let output = context.run(egui::RawInput {
			events: vec![egui::Event::PointerMoved(egui::pos2(60.0, 60.0))],
			..Default::default()
		}, |context| {
			egui::CentralPanel::default().show(context, |ui| {
				ui.label("Pointer preview");
			});
			context.set_cursor_icon(cursor);
			finish_frame(context, false, None, false, true);
		});
		let expected = if matches!(cursor, egui::CursorIcon::Default | egui::CursorIcon::PointingHand) {
			egui::CursorIcon::None
		}
		else {
			cursor
		};
		assert_eq!(output.platform_output.cursor_icon, expected);
	}
}

#[test]
fn channels_fit_the_minimum_main_area_and_scroll_in_narrow_project_windows() {
	for size in [vec2(559.0, 300.0), vec2(228.0, 300.0)] {
		let context = egui::Context::default();
		theme::apply(&context, ID);
		let _ = context.run(egui::RawInput {
			screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, size)),
			..Default::default()
		}, |context| {
			egui::CentralPanel::default().frame(Frame::NONE).show(context, |ui| {
				home_channels(ui, "What can I help with?");
				assert!(ui.min_rect().width() <= size.x + 0.5);
				assert!(ui.min_rect().height() <= size.y + 0.5);
			});
		});
	}
}
