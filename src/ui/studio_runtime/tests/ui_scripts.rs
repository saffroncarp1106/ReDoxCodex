use redox_studio::{
    actions::UiAction,
    document::{Document, Kind, Node},
};

#[test]
fn old_documents_and_bound_wii_round_trip() {
    for (id, _) in redox_studio::originals::THEMES {
        let doc = redox_studio::originals::load(id).unwrap();
        assert_eq!(Document::parse(&doc.encode().unwrap()).unwrap(), doc);
    }
    let doc = redox_studio::originals::load("wii-controls").unwrap();
    for action in [
        UiAction::History,
        UiAction::Send,
        UiAction::Effort,
        UiAction::Composer,
    ] {
        assert!(
            doc.pages[0]
                .nodes
                .iter()
                .any(|n| n.ui_script.contains(&action))
        );
    }
}

#[test]
fn scripts_are_bounded_and_unknown_calls_fail() {
    let mut doc = Document::blank();
    let mut node = Node::new(1, Kind::Button, 0., 0.);
    node.ui_script = vec![UiAction::History; 17];
    doc.pages[0].nodes.push(node);
    assert!(doc.validate().is_err());
    doc.pages[0].nodes[0].ui_script = vec![UiAction::Composer, UiAction::Send];
    assert!(doc.validate().is_err());
    doc.pages[0].nodes[0].ui_script = vec![UiAction::History, UiAction::Chats];
    let mut value = serde_json::to_value(&doc).unwrap();
    value["pages"][0]["nodes"][0]["ui_script"] = serde_json::json!(["execute_shell"]);
    assert!(Document::parse(&serde_json::to_vec(&value).unwrap()).is_err());
}

#[test]
fn replacing_an_asset_keeps_binding_and_reclaims_old_texture() {
    let mut doc = Document::blank();
    let mut node = Node::new(1, Kind::Button, 0., 0.);
    node.ui_script = vec![UiAction::Send];
    doc.pages[0].nodes.push(node);
    let image = image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 255, 255, 255]));
    let mut bytes = std::io::Cursor::new(vec![]);
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    for _ in 0..12 {
        redox_studio::assets::replace_png(&mut doc, 0, 1, bytes.get_ref()).unwrap();
    }
    assert_eq!(doc.textures.len(), 1);
    assert_eq!(doc.pages[0].nodes[0].ui_script, vec![UiAction::Send]);
    assert_eq!(Document::parse(&doc.encode().unwrap()).unwrap(), doc);
    let before = doc.clone();
    assert!(redox_studio::assets::replace_png(&mut doc, 0, 1, b"invalid").is_err());
    assert_eq!(doc, before);
}

#[test]
fn converting_artwork_to_editable_styling_keeps_calls_and_supports_undo() {
    use redox_studio::editor::Editor;
    let doc = redox_studio::originals::load("win98").unwrap();
    let mut editor = Editor::new(doc.clone());
    let id = doc.pages[0]
        .nodes
        .iter()
        .find(|n| n.kind == Kind::Artwork)
        .unwrap()
        .id;
    editor.begin();
    editor.page_mut().node_mut(id).unwrap().ui_script = vec![UiAction::History];
    editor.commit();
    let before = editor.document.clone();
    editor.begin();
    redox_studio::assets::detach(&mut editor.document, 0, id).unwrap();
    editor.commit();
    let node = editor.page().node(id).unwrap();
    assert_eq!(node.kind, Kind::Rectangle);
    assert!(node.artwork.is_none());
    assert_eq!(node.ui_script, vec![UiAction::History]);
    assert_eq!(editor.page().world_rect(id), before.pages[0].world_rect(id));
    editor.document.validate().unwrap();
    editor.undo();
    assert_eq!(editor.document, before);
    editor.redo();
    assert!(editor.page().node(id).unwrap().artwork.is_none());
}

#[test]
fn png_replacement_keeps_editable_captions_and_live_picker_values_above_the_image() {
    use eframe::egui::{self, Pos2, Rect, Vec2};
    for kind in [Kind::Button, Kind::Text] {
        let mut doc = Document::blank();
        let mut node = Node::new(1, kind, 0., 0.);
        node.text = "Old model".into();
        node.ui_script = vec![UiAction::Model];
        doc.pages[0].nodes = vec![node];
        let image = image::RgbaImage::from_pixel(4, 4, image::Rgba([50, 100, 200, 255]));
        let mut bytes = std::io::Cursor::new(vec![]);
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        redox_studio::assets::replace_png(&mut doc, 0, 1, bytes.get_ref()).unwrap();
        let ctx = egui::Context::default();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800., 600.))),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    redox_studio::presentation::interactive::show_with_values(
                        ui,
                        &doc,
                        redox_studio::presentation::interactive::Values {
                            model: Some("Live model"),
                            ..Default::default()
                        },
                        &mut |_, _| {},
                    );
                });
            },
        );
        assert!(
            output
                .shapes
                .iter()
                .any(|s| matches!(&s.shape, egui::Shape::Mesh(_))),
            "The replacement image must be drawn"
        );
        assert_eq!(
            output
                .shapes
                .iter()
                .filter(
                    |s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "Live model")
                )
                .count(),
            1
        );
        assert!(
            !output.shapes.iter().any(
                |s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "Old model")
            )
        );
        assert_eq!(doc.pages[0].node(1).unwrap().ui_script, [UiAction::Model]);
        let editing = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800., 600.))),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    redox_studio::presentation::scene::render(
                        ui.painter(),
                        &doc.pages[0],
                        redox_studio::presentation::scene::View {
                            origin: Pos2::ZERO,
                            zoom: 1.,
                        },
                        Some(1),
                        &doc.textures,
                    )
                });
            },
        );
        assert!(
            editing
                .shapes
                .iter()
                .any(|s| matches!(&s.shape, egui::Shape::Mesh(_))),
            "Editing a caption must keep its image background"
        );
    }
}

#[test]
fn interactive_scene_dispatches_topmost_visible_call() {
    use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, Rect, Vec2};
    let mut doc = Document::blank();
    for (id, action) in [(1, UiAction::History), (2, UiAction::Settings)] {
        let mut node = Node::new(id, Kind::Button, 0., 0.);
        node.ui_script = vec![action];
        doc.pages[0].nodes.push(node);
    }
    let ctx = egui::Context::default();
    let mut calls = vec![];
    let point = Pos2::new(40., 30.);
    for events in [
        vec![],
        vec![
            Event::PointerMoved(point),
            Event::PointerButton {
                pos: point,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
        vec![Event::PointerButton {
            pos: point,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    ] {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800., 600.))),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    calls.extend(redox_studio::presentation::interactive::show(
                        ui,
                        &doc,
                        &mut |_, _| {},
                    ));
                });
            },
        );
    }
    assert_eq!(calls, vec![UiAction::Settings]);
}

#[test]
fn controls_in_different_viewports_have_independent_focus_and_popup_ids() {
    use eframe::egui::{self, Pos2, Rect, Vec2};
    let doc = {
        let mut doc = Document::blank();
        let mut node = Node::new(1, Kind::Button, 0., 0.);
        node.ui_script = vec![UiAction::Menus];
        doc.pages[0].nodes.push(node);
        doc
    };
    let ctx = egui::Context::default();
    let mut ids = vec![];
    for viewport in [
        egui::ViewportId::ROOT,
        egui::ViewportId::from_hash_of("detached-preview"),
    ] {
        let mut input = egui::RawInput {
            viewport_id: viewport,
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800., 600.))),
            ..Default::default()
        };
        input.viewports.insert(viewport, Default::default());
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                redox_studio::presentation::interactive::show_with_values(
                    ui,
                    &doc,
                    Default::default(),
                    &mut |_, control| ids.push(control.response.id),
                );
            });
        });
    }
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
}

#[test]
fn interactive_fit_does_not_force_large_designs_to_overflow_the_viewport() {
    use eframe::egui::{self, Pos2, Rect, Vec2};
    let ctx = egui::Context::default();
    let mut doc = Document::blank();
    let mut frame = Node::new(1, Kind::Frame, 0., 0.);
    frame.width = 16000.;
    frame.height = 16000.;
    let mut button = Node::new(2, Kind::Button, 0., 0.);
    button.parent = Some(1);
    button.ui_script = vec![UiAction::Menus];
    doc.pages[0].nodes = vec![frame, button];
    let mut metrics = None;
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(360., 320.))),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let width = ui.available_width();
                redox_studio::presentation::interactive::show_with_values(
                    ui,
                    &doc,
                    Default::default(),
                    &mut |_, control| {
                        metrics = Some((width, control.zoom));
                    },
                );
            });
        },
    );
    let (available, zoom) = metrics.unwrap();
    assert!(16000. * zoom <= available + 0.01);
    assert!(zoom < 0.05);
}

#[test]
fn composer_has_one_tab_stop_and_focuses_the_text_editor() {
    use eframe::egui::{self, Event, Key, Modifiers, Pos2, Rect, Vec2};
    let ctx = egui::Context::default();
    let mut doc = Document::blank();
    let mut node = Node::new(1, Kind::Input, 0., 0.);
    node.ui_script = vec![UiAction::Composer];
    doc.pages[0].nodes.push(node);
    let mut draft = String::new();
    let mut editor_id = None;
    for events in [
        vec![],
        vec![],
        vec![Event::Key {
            key: Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        vec![],
    ] {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800., 600.))),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    redox_studio::presentation::interactive::show_with_values(
                        ui,
                        &doc,
                        Default::default(),
                        &mut |ui, control| {
                            let result = redox_studio::presentation::interactive::composer(
                                ui,
                                control,
                                &mut draft,
                                egui::Id::new("test-chat"),
                                false,
                            );
                            editor_id = Some(result.response.id);
                        },
                    );
                });
            },
        );
    }
    assert_eq!(ctx.memory(|m| m.focused()), editor_id);
}

#[test]
fn changing_a_captured_labels_font_family_invalidates_its_original_glyph_mesh() {
    use eframe::egui::{self, Pos2, Rect, Vec2};
    let mut doc = redox_studio::originals::load("win98").unwrap();
    let node = doc.pages[0]
        .nodes
        .iter_mut()
        .find(|n| n.kind == Kind::Text && n.artwork.as_ref().is_some_and(|a| !a.meshes.is_empty()))
        .unwrap();
    let label = node.text.clone();
    node.font = redox_studio::document::Font::Mono;
    node.artwork.as_mut().unwrap().original_font = redox_studio::document::Font::Sans;
    let ctx = egui::Context::default();
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 900.))),
            max_texture_side: Some(4096),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                redox_studio::presentation::scene::render(
                    ui.painter(),
                    &doc.pages[0],
                    redox_studio::presentation::scene::View {
                        origin: Pos2::ZERO,
                        zoom: 1.,
                    },
                    None,
                    &doc.textures,
                )
            });
        },
    );
    let text = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == label => Some(text),
            _ => None,
        })
        .expect("Font changes must produce a new galley rather than replaying the old glyphs");
    assert_eq!(
        text.galley.job.sections[0].format.font_id.family,
        egui::FontFamily::Monospace
    );
}

#[test]
fn resizing_and_aligning_captured_text_reflows_it_without_stretching_glyphs() {
    use eframe::egui::{self, Pos2, Rect, Vec2};
    use redox_studio::document::TextAlign;
    let mut doc = redox_studio::originals::load("win98").unwrap();
    let label = doc.pages[0]
        .nodes
        .iter_mut()
        .find(|node| {
            node.kind == Kind::Text && node.artwork.as_ref().is_some_and(|a| !a.meshes.is_empty())
        })
        .unwrap();
    label.text = "A resized captured label now wraps into several lines".into();
    label.x = 10.;
    label.y = 20.;
    label.parent = None;
    label.width = 130.;
    label.height = 90.;
    label.font_size = 16.;
    label.text_align = TextAlign::Right;
    let label = label.clone();
    doc.pages[0].nodes = vec![label.clone()];
    let ctx = egui::Context::default();
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(500.))),
            max_texture_side: Some(4096),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                redox_studio::presentation::scene::render(
                    ui.painter(),
                    &doc.pages[0],
                    redox_studio::presentation::scene::View {
                        origin: Pos2::ZERO,
                        zoom: 1.,
                    },
                    None,
                    &doc.textures,
                )
            });
        },
    );
    let (shape, text) = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == label.text => Some((shape, text)),
            _ => None,
        })
        .unwrap();
    assert!(text.galley.rows.len() > 1);
    assert_eq!(text.galley.job.halign, egui::Align::Max);
    assert_eq!(text.galley.job.sections[0].format.font_id.size, 16.);
    assert!(shape.clip_rect.width() <= label.width);
    assert!(shape.clip_rect.height() <= label.height);
    assert!((text.pos.x - 140.).abs() < 0.01);
}

#[test]
fn alignment_and_bounds_edits_invalidate_captured_glyphs_even_when_text_is_unchanged() {
    use eframe::egui::{self, Pos2, Rect, Vec2};
    use redox_studio::document::TextAlign;
    let doc = redox_studio::originals::load("win98").unwrap();
    let original = doc.pages[0]
        .nodes
        .iter()
        .find(|node| {
            node.kind == Kind::Text && node.artwork.as_ref().is_some_and(|a| !a.meshes.is_empty())
        })
        .unwrap();
    for resize in [false, true] {
        let mut edited = doc.clone();
        let node = edited.pages[0].node_mut(original.id).unwrap();
        if resize {
            node.width += 150.;
        } else {
            node.text_align = TextAlign::Right;
        }
        let ctx = egui::Context::default();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 900.))),
                max_texture_side: Some(4096),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    redox_studio::presentation::scene::render(
                        ui.painter(),
                        &edited.pages[0],
                        redox_studio::presentation::scene::View {
                            origin: Pos2::ZERO,
                            zoom: 1.,
                        },
                        None,
                        &edited.textures,
                    )
                });
            },
        );
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == original.text)),
            "Changing {} must relayout the original label", if resize { "bounds" } else { "alignment" });
    }
}

#[test]
fn unchanged_source_text_uses_live_fonts_at_each_zoom_and_monitor_scale() {
    use eframe::egui::{self, Pos2, Rect, Vec2};
    let mut doc = redox_studio::originals::load("win98").unwrap();
    let mut label = doc.pages[0]
        .nodes
        .iter()
        .find(|n| n.kind == Kind::Text && n.artwork.as_ref().is_some_and(|a| !a.meshes.is_empty()))
        .unwrap()
        .clone();
    label.parent = None;
    label.x = 10.;
    label.y = 10.;
    doc.pages[0].nodes = vec![label.clone()];
    let ctx = egui::Context::default();
    redox_studio::presentation::typography::install(&ctx);
    for (dpi, zoom) in [(1., 0.5), (1., 1.), (1.5, 2.), (2., 4.)] {
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(1600.))),
            max_texture_side: Some(4096),
            ..Default::default()
        };
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(dpi);
        let output = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                redox_studio::presentation::scene::render(
                    ui.painter(),
                    &doc.pages[0],
                    redox_studio::presentation::scene::View {
                        origin: Pos2::ZERO,
                        zoom,
                    },
                    None,
                    &doc.textures,
                )
            });
        });
        let text = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == label.text => Some(t),
                _ => None,
            })
            .expect("Unedited captured text must use fresh font layout");
        assert_eq!(
            text.galley.job.sections[0].format.font_id.size,
            label.font_size * zoom
        );
        assert!(
            !output
                .shapes
                .iter()
                .any(|s| matches!(s.shape, egui::Shape::Mesh(_))),
            "Text must not replay the captured texture mesh"
        );
        assert_eq!(ctx.pixels_per_point(), dpi);
        assert_eq!(doc.pages[0].nodes[0], label);
    }
}
