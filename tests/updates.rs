use redox::updates::newer;

#[test]
fn stable_releases_use_numeric_version_order() {
    assert!(newer("v0.1.10", "0.1.4"));
    assert!(!newer("v0.1.4", "0.1.4"));
    assert!(!newer("0.1.3", "0.1.4"));
    assert!(!newer("v0.2.0-beta", "0.1.4"));
    assert!(!newer("garbage", "0.1.4"));
}
