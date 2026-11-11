use redox_studio::{
    actions::UiAction,
    document::{Document, Kind, Node},
    scenes::SceneId,
    simulation::Simulation,
};

#[test]
fn simulation_sends_locally_and_visits_every_settings_scene() {
    let mut sim = Simulation::default();
    sim.reset_for(&Document::blank());
    sim.draft = "Dummy text".into();
    sim.dispatch(UiAction::Send);
    assert_eq!(sim.scene, SceneId::Conversation);
    assert_eq!(sim.messages.len(), 1);
    assert!(sim.messages[0].1.contains("Yatta yatta"));
    assert!(sim.draft.is_empty());
    for (action, scene) in [
        (UiAction::SettingsAccount, SceneId::Account),
        (UiAction::SettingsModels, SceneId::Models),
        (UiAction::SettingsWorkspace, SceneId::Workspace),
        (UiAction::SettingsAppearance, SceneId::Appearance),
        (UiAction::SettingsCursors, SceneId::Cursors),
        (UiAction::SettingsAdvanced, SceneId::Advanced),
        (UiAction::SettingsUpdates, SceneId::Updates),
        (UiAction::SettingsAbout, SceneId::About),
    ] {
        sim.dispatch(action);
        assert_eq!(sim.scene, scene);
    }
    sim.dispatch(UiAction::SettingsClose);
    assert_eq!(sim.scene, SceneId::Conversation);
    sim.dispatch(UiAction::NewChat);
    assert_eq!(sim.scene, SceneId::Welcome);
    assert!(sim.messages.is_empty());
}

#[test]
fn live_inspector_changes_reach_simulation_without_saving() {
    let mut doc = Document::blank();
    doc.pages[0].nodes.push(Node::new(1, Kind::Text, 0., 0.));
    let mut sim = Simulation::default();
    sim.reset_for(&doc);
    doc.pages[0].nodes[0].text = "Changed during play".into();
    assert_eq!(
        sim.document(&doc).unwrap().pages[0].nodes[0].text,
        "Changed during play"
    );
    sim.draft = "Keep my draft".into();
    sim.document(&doc).unwrap();
    assert_eq!(sim.draft, "Keep my draft");
}

#[test]
fn all_themes_have_all_source_settings_scenes() {
    for theme in redox_studio::scenes::THEMES {
        for scene in SceneId::ALL {
            let doc = redox_studio::scenes::load(theme, scene).unwrap();
            let source = doc.source.as_ref().unwrap();
            assert_eq!(source.theme, theme);
            assert_eq!(source.scene, scene);
            assert!(doc.pages[0].nodes.iter().any(|n| n.artwork.is_some()));
            if matches!(scene, SceneId::Welcome | SceneId::Conversation) {
                for call in [UiAction::Composer, UiAction::Menus, UiAction::ModelEffort] {
                    assert!(
                        doc.pages[0].nodes.iter().any(|n| n.ui_script == vec![call]),
                        "Missing {} in {} {}",
                        call.id(),
                        theme,
                        scene.id()
                    );
                }
            }
            doc.validate().unwrap();
        }
    }
    assert!(redox_studio::scenes::load("../other", SceneId::About).is_err());
}

#[test]
fn editing_after_navigation_returns_to_the_live_layout_and_keeps_the_session() {
    let mut doc = redox_studio::scenes::load("codex", SceneId::Welcome).unwrap();
    let mut sim = Simulation::default();
    sim.reset_for(&doc);
    sim.draft = "Keep this draft".into();
    sim.dispatch(UiAction::Settings);
    assert_eq!(
        sim.document(&doc).unwrap().source.unwrap().scene,
        SceneId::Account
    );
    doc.pages[0].nodes[0].fill = [10, 20, 30, 255];
    let live = sim.document(&doc).unwrap();
    assert_eq!(live, doc);
    assert_eq!(sim.scene, SceneId::Welcome);
    assert_eq!(sim.draft, "Keep this draft");
    sim.dispatch(UiAction::Settings);
    assert_eq!(
        sim.document(&doc).unwrap().source.unwrap().scene,
        SceneId::Account
    );
    sim.dispatch(UiAction::SettingsClose);
    assert_eq!(sim.document(&doc).unwrap(), doc);
}

#[test]
fn custom_layout_is_retained_when_sending_and_undoing_edits() {
    let mut doc = Document::blank();
    let mut button = Node::new(1, Kind::Button, 30., 40.);
    button.text = "Custom menu".into();
    button.ui_script = vec![UiAction::Menus];
    doc.pages[0].nodes.push(button);
    let mut editor = redox_studio::editor::Editor::new(doc.clone());
    let mut sim = Simulation::default();
    sim.reset_for(&doc);
    sim.draft = "Try my custom layout".into();
    sim.dispatch(UiAction::Send);
    assert_eq!(sim.document(&doc).unwrap(), doc);
    editor.begin();
    editor.document.pages[0].nodes[0].text = "Edited menu".into();
    editor.document.pages[0].nodes[0].x = 110.;
    editor.commit();
    assert_eq!(
        sim.document(&editor.document).unwrap().pages[0].nodes[0].text,
        "Edited menu"
    );
    editor.undo();
    assert_eq!(sim.document(&editor.document).unwrap(), doc);
    editor.redo();
    assert_eq!(
        sim.document(&editor.document).unwrap().pages[0].nodes[0].x,
        110.
    );
    assert_eq!(sim.messages.len(), 1);
}

#[test]
fn authored_conversation_text_is_not_overwritten_by_fixture_substitution() {
    let mut doc = redox_studio::scenes::load("codex", SceneId::Conversation).unwrap();
    let node = doc.pages[0]
        .nodes
        .iter_mut()
        .find(|n| n.text.starts_with("Dummy text."))
        .unwrap();
    node.text = "Dummy text. My authored example.".into();
    let id = node.id;
    let mut sim = Simulation::default();
    sim.reset_for(&doc);
    sim.draft = "A different prompt".into();
    sim.dispatch(UiAction::Send);
    assert_eq!(
        sim.document(&doc).unwrap().pages[0].node(id).unwrap().text,
        "Dummy text. My authored example."
    );
}

#[test]
fn preview_page_and_nested_frame_selection_keep_the_correct_assets_and_coordinates() {
    let mut doc = Document::blank();
    let mut second = redox_studio::document::Page::blank("Second page");
    let mut outer = Node::new(1, Kind::Frame, 100., 200.);
    outer.width = 500.;
    outer.height = 400.;
    let mut inner = Node::new(2, Kind::Frame, 30., 40.);
    inner.parent = Some(1);
    let mut child = Node::new(3, Kind::Button, 10., 20.);
    child.parent = Some(2);
    second.nodes = vec![outer, inner, child];
    doc.pages.push(second);
    let selected =
        redox_studio::presentation::interactive::selected_page(&doc, 1, Some(2)).unwrap();
    assert_eq!(selected.pages[0].nodes.len(), 2);
    assert_eq!(selected.pages[0].node(2).unwrap().parent, None);
    assert_eq!(selected.pages[0].world_rect(3).unwrap()[..2], [140., 260.]);
    assert_eq!(doc.pages[1].node(2).unwrap().parent, Some(1));
    assert!(redox_studio::presentation::interactive::selected_page(&doc, 0, Some(2)).is_err());
}

#[test]
fn functional_components_use_editor_validation_and_preserve_undo() {
    use redox_studio::{document::MAX_NODES, editor::Editor};
    let mut editor = Editor::new(Document::blank());
    for action in UiAction::ALL {
        let id = editor.add_component(*action, [30., 40.], None).unwrap();
        let node = editor.page().node(id).unwrap();
        assert_eq!(node.ui_script, vec![*action]);
        assert_eq!(
            node.kind,
            match action {
                UiAction::Composer => Kind::Input,
                UiAction::Conversation => Kind::Frame,
                _ => Kind::Button,
            }
        );
    }
    editor.document.validate().unwrap();
    let before = editor.document.clone();
    assert!(
        editor
            .add_component(UiAction::Send, [f32::NAN, 0.], None)
            .is_err()
    );
    assert_eq!(editor.document, before);
    while editor.document.node_count() < MAX_NODES {
        editor
            .add(Kind::Rectangle, [0., 0., 10., 10.], None)
            .unwrap();
    }
    let before = editor.document.clone();
    assert!(
        editor
            .add_component(UiAction::Send, [0., 0.], None)
            .is_err()
    );
    assert_eq!(editor.document, before);
}

#[test]
fn settings_close_returns_to_the_chat_selected_from_the_scene_picker() {
    let mut sim = Simulation::default();
    sim.reset_for(&Document::blank());
    sim.navigate(SceneId::Conversation);
    sim.navigate(SceneId::About);
    sim.dispatch(UiAction::SettingsModels);
    sim.dispatch(UiAction::SettingsClose);
    assert_eq!(sim.scene, SceneId::Conversation);
    sim.navigate(SceneId::Welcome);
    sim.navigate(SceneId::Appearance);
    sim.dispatch(UiAction::SettingsClose);
    assert_eq!(sim.scene, SceneId::Welcome);
}

#[test]
fn custom_layout_can_experiment_with_another_settings_theme_without_losing_edits_or_chat() {
    let mut doc = redox_studio::originals::load("wii-live-workspace").unwrap();
    let mut sim = Simulation::default();
    sim.reset_for(&doc);
    sim.set_reference_theme("wii").unwrap();
    sim.draft = "Keep this prompt".into();
    sim.dispatch(UiAction::Send);
    sim.dispatch(UiAction::SettingsAbout);
    assert_eq!(sim.document(&doc).unwrap().source.unwrap().theme, "wii");
    doc.pages[0].nodes[0].fill = [230, 240, 250, 255];
    sim.sync_editor(&doc);
    assert_eq!(sim.reference_theme(), "wii");
    assert_eq!(sim.messages.len(), 1);
    assert_eq!(sim.document(&doc).unwrap(), doc);
    sim.dispatch(UiAction::SettingsModels);
    let settings = sim.document(&doc).unwrap();
    assert_eq!(settings.source.unwrap().theme, "wii");
    assert!(sim.set_reference_theme("../../unknown").is_err());
    assert_eq!(sim.reference_theme(), "wii");
}
