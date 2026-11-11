use super::*;

#[test]
fn cached_scene_reloads_external_edits_and_keeps_the_last_valid_copy_during_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scene.redoxskin");
    let mut doc = Document::blank();
    doc.save(&path).unwrap();
    let mut cache = CachedScene::load(path.clone()).unwrap();
    doc.name = "Externally edited preview".into();
    doc.save(&path).unwrap();
    cache.checked = std::time::Instant::now() - std::time::Duration::from_secs(1);
    assert!(cache.refresh().is_none());
    assert_eq!(cache.document, doc);
    std::fs::remove_file(&path).unwrap();
    cache.checked = std::time::Instant::now() - std::time::Duration::from_secs(1);
    assert!(cache.refresh().is_some());
    assert_eq!(cache.document, doc);
    std::fs::write(&path, b"partial file").unwrap();
    cache.checked = std::time::Instant::now() - std::time::Duration::from_secs(1);
    assert!(cache.refresh().is_some());
    assert_eq!(cache.document, doc);
    doc.name = "Recovered source scene".into();
    doc.save(&path).unwrap();
    cache.checked = std::time::Instant::now() - std::time::Duration::from_secs(1);
    assert!(cache.refresh().is_none());
    assert_eq!(cache.document, doc);
}
