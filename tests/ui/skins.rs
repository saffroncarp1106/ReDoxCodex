use super::*;

#[test]
fn cloning_a_skin_preserves_its_picture_opacity() {
    let root = std::env::temp_dir().join(format!("skin-editor-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let picture = root.join("picture.png");
    image::RgbImage::new(8, 4).save(&picture).unwrap();
    let pack = packs::Pack {
        id: "existing".into(),
        path: root.join("ui.json"),
        config: packs::Config {
            background_image: Some("picture.png".into()),
            background_opacity: 0.7,
            ..packs::Config::default()
        },
    };
    let context = egui::Context::default();
    let mut editor = Editor::from_pack(&pack, &context);
    assert_eq!(editor.config.background_opacity, 0.7);
    assert!(editor.texture.is_some());
    editor.choose_image(root.join("missing.png"), &context);
    assert_eq!(
        editor.image.as_ref(),
        Some(&picture.canonicalize().unwrap())
    );
    assert_eq!(editor.config.background_opacity, 0.7);
    assert!(editor.error.is_some());
}
