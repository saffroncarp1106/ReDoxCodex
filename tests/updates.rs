use redox::updates::newer;

#[test]
fn stable_releases_use_numeric_version_order() {
    assert!(newer("v0.1.10", "0.1.4"));
    assert!(newer("v0.1.5", "0.1.4"));
    assert!(!newer("v0.1.5", "0.1.5"));
    assert!(!newer("v0.1.4", "0.1.4"));
    assert!(!newer("0.1.3", "0.1.4"));
    assert!(!newer("v0.2.0-beta", "0.1.4"));
    assert!(!newer("garbage", "0.1.4"));
}

#[test]
fn release_assets_and_manual_checks_start_idle() {
    let release: redox::updates::Release = serde_json::from_value(serde_json::json!({
        "tag_name": "v0.1.6",
        "html_url": "https://github.com/saffroncarp1106/ReDoxCodex/releases/tag/v0.1.6",
        "assets": [{"name": "ReDox-Windows-x64.exe", "url": "https://api.github.com/repos/saffroncarp1106/ReDoxCodex/releases/assets/123", "size": 100}]
    })).unwrap();
    assert!(newer(&release.tag_name, "0.1.5"));
    assert_eq!(release.assets[0].name, "ReDox-Windows-x64.exe");
    let updates = redox::updates::Updates::default();
    assert!(!updates.checking());
    assert!(!updates.startup_checked);
}
