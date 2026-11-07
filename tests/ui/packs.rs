use super::*;

fn directory() -> PathBuf {
    let path = std::env::temp_dir().join(format!("codex-v011-pack-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn builtins_and_new_configs_are_discovered() {
    let root = directory();
    std::fs::write(
        root.join("custom.json"),
        r#"{"name":"Ocean","layout":"gemini"}"#,
    )
    .unwrap();
    let catalog = discover(&root);
    assert!(catalog.packs.iter().any(|pack| pack.id == "codex"));
    assert!(catalog.packs.iter().any(|pack| pack.id == "gemini"));
    assert!(catalog.packs.iter().any(|pack| pack.id == "claude"));
    assert!(catalog.packs.iter().any(|pack| pack.config.name == "Ocean"));
}

#[test]
fn missing_assets_are_reported_and_import_does_not_overwrite() {
    let root = directory();
    let source = root.join("ui.json");
    std::fs::write(
        &source,
        r#"{"name":"Ocean","background_image":"absent.png"}"#,
    )
    .unwrap();
    assert!(read(&source).is_err());
    std::fs::write(&source, r#"{"name":"Ocean"}"#).unwrap();
    let destination = directory();
    let first = install(&source, &destination).unwrap();
    let second = install(&source, &destination).unwrap();
    assert_ne!(first, second);
}

#[test]
fn assets_cannot_leave_the_pack_folder() {
    assert!(asset_path(&directory().join("ui.json"), "../outside.png").is_err());
}

#[test]
fn arbitrary_names_and_layouts_survive_import_and_discovery() {
    let source = directory().join("anything.json");
    std::fs::write(&source, r#"{"name":"My own UI","layout":"my-new-ui"}"#).unwrap();
    let destination = directory();
    let id = install(&source, &destination).unwrap();
    let catalog = discover(&destination);
    assert!(catalog.errors.is_empty());
    let pack = catalog.packs.iter().find(|pack| pack.id == id).unwrap();
    assert_eq!(pack.config.name, "My own UI");
    assert_eq!(pack.config.layout, "my-new-ui");
}

#[test]
fn imports_cannot_shadow_embedded_layouts() {
    let source = directory().join("ui.json");
    let destination = directory();
    for renderer in super::super::renderers::RENDERERS {
        std::fs::write(&source, renderer.config).unwrap();
        let id = install(&source, &destination).unwrap();
        assert_ne!(id, renderer.id);
    }
}

#[test]
fn visual_skin_save_round_trips_picture_and_opacity_without_source() {
    let source = directory().join("photo.jpg");
    image::RgbImage::from_pixel(16, 8, image::Rgb([24, 100, 180]))
        .save(&source)
        .unwrap();
    let root = directory();
    let config = Config {
        name: "My Claude skin".into(),
        layout: "claude".into(),
        background_opacity: 0.35,
        ..Config::default()
    };
    let id = save(&config, Some(&source), &root).unwrap();
    let pack = root.join(id).join("ui.json");
    let saved = read(&pack).unwrap();
    assert_eq!(saved.layout, "claude");
    assert_eq!(saved.background_opacity, 0.35);
    assert_eq!(saved.background_image.as_deref(), Some("background.png"));
    assert_ne!(asset_path(&pack, "background.png").unwrap(), source);
    let shared = directory();
    let shared_id = install(&pack, &shared).unwrap();
    assert!(read(&shared.join(shared_id).join("ui.json")).is_ok());
}

#[test]
fn invalid_picture_or_settings_do_not_create_a_skin() {
    let root = directory();
    let source = directory().join("broken.png");
    std::fs::write(&source, b"not an image").unwrap();
    assert!(save(&Config::default(), Some(&source), &root).is_err());
    let config = Config {
        background_opacity: f32::NAN,
        ..Config::default()
    };
    assert!(save(&config, None, &root).is_err());
    assert_eq!(std::fs::read_dir(root).unwrap().count(), 0);
}

#[test]
fn jpeg_webp_and_gif_backgrounds_are_accepted() {
    let root = directory();
    for extension in ["jpg", "webp", "gif", "PNG"] {
        let image = root.join(format!("picture.{extension}"));
        image::RgbImage::from_pixel(4, 4, image::Rgb([40, 60, 80]))
            .save(&image)
            .unwrap();
        let config = Config {
            background_image: Some(format!("picture.{extension}")),
            ..Config::default()
        };
        assert!(
            config.validate(&root.join("ui.json")).is_ok(),
            "{extension}"
        );
    }
}

#[test]
fn oversized_picture_dimensions_are_rejected() {
    let source = directory().join("wide.png");
    image::RgbImage::new(4097, 1).save(&source).unwrap();
    assert!(load_background(&source).unwrap_err().contains("4096"));
}

#[test]
fn old_packs_keep_full_picture_opacity() {
    let config: Config = serde_json::from_str(r#"{"name":"Existing"}"#).unwrap();
    assert_eq!(config.background_opacity, 1.0);
}
