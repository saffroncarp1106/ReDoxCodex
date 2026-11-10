use super::*;

#[test]
fn registered_layouts_use_their_own_callbacks() {
    for (id, rail, conversation) in [
        (
            "codex",
            ReDox::navigation_rail as fn(&mut ReDox, &egui::Context),
            ReDox::conversation as fn(&mut ReDox, &egui::Context),
        ),
        ("gemini", ReDox::gemini_rail, ReDox::gemini_conversation),
        ("claude", ReDox::claude_rail, ReDox::claude_conversation),
		("wii", ReDox::wii_rail, ReDox::wii_conversation),
        ("win98", ReDox::win98_desktop, ReDox::win98_conversation),
    ] {
        let renderer = resolve(id);
        assert_eq!(renderer.id, id);
        assert!(std::ptr::fn_addr_eq(renderer.rail, rail));
        assert!(std::ptr::fn_addr_eq(renderer.conversation, conversation));
    }
}

#[test]
fn unknown_layout_uses_default_renderer() {
    assert_eq!(resolve("my-new-ui").id, "codex");
    assert_eq!(resolve("").id, "codex");
}

#[test]
fn embedded_configs_have_unique_ids_and_matching_renderers() {
    let mut ids = std::collections::BTreeSet::new();
    for renderer in RENDERERS {
        assert!(ids.insert(renderer.id));
        let config: super::super::packs::Config = serde_json::from_str(renderer.config).unwrap();
        assert_eq!(config.layout, renderer.id);
        config.validate(std::path::Path::new("ui.json")).unwrap();
    }
}
