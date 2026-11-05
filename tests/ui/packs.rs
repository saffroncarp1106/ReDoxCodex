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
