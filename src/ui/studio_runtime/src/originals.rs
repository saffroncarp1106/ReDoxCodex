use crate::document::Document;

pub const THEMES: [(&str, &str); 8] = [
    ("codex", "Codex"),
    ("claude", "Claude"),
    ("gemini", "Gemini"),
    ("win98", "Windows 98"),
    ("wii", "Wii"),
    ("wii-controls", "Wii functional controls"),
    ("asset-controls", "Wii asset controls"),
    ("wii-live-workspace", "Wii live workspace"),
];

pub fn load(id: &str) -> Result<Document, String> {
    if let Some((theme, scene)) = id.split_once(':') {
        let scene = crate::scenes::SceneId::ALL
            .into_iter()
            .find(|s| s.id() == scene)
            .ok_or("Unknown source scene")?;
        return crate::scenes::load(theme, scene);
    }
    let bytes: &[u8] = match id {
        "codex" => include_bytes!("../skins/redox/codex.redoxskin"),
        "claude" => include_bytes!("../skins/redox/claude.redoxskin"),
        "gemini" => include_bytes!("../skins/redox/gemini.redoxskin"),
        "win98" => include_bytes!("../skins/redox/win98.redoxskin"),
        "asset-controls" => include_bytes!("../skins/redox/asset-controls.redoxskin"),
        "wii-live-workspace" => include_bytes!("../skins/redox/wii-live-workspace.redoxskin"),
        "wii-controls" => include_bytes!("../skins/redox/wii-controls.redoxskin"),
        "wii" => include_bytes!("../skins/redox/wii.redoxskin"),
        _ => return Err("Unknown ReDox original".into()),
    };
    Document::parse(bytes)
}

#[cfg(test)]
mod tests {
    #[test]
    fn wii_source_document_is_editable_and_renders() {
        let doc = super::load("wii").unwrap();
        assert!(doc.source.as_ref().is_some_and(|s| s.theme == "wii"));
        assert!(doc.pages[0].nodes.iter().any(|n| !n.text.is_empty()));
        assert!(doc.pages[0].nodes.iter().any(|n| !n.ui_script.is_empty()));
        let png =
            crate::presentation::render_document(&doc, 0, Some(1), 1100, 900, || false).unwrap();
        assert_eq!(image::load_from_memory(&png).unwrap().width(), 1100);
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/ui-review");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("wii-native.png"), png).unwrap();
    }
}
