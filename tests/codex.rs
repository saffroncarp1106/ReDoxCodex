#![cfg(target_os = "macos")]

use redox::codex::cli_executable;

#[test]
fn mac_app_paths_resolve_to_the_cli_and_reject_desktop_launchers() {
    let root = std::env::temp_dir().join(format!("codex-cli-{}", uuid::Uuid::new_v4()));
    let app = root.join("Codex.app");
    let contents = app.join("Contents");
    let resources = contents.join("Resources");
    let launcher = contents.join("MacOS/Codex");
    let cli = resources.join("codex");
    std::fs::create_dir_all(launcher.parent().unwrap()).unwrap();
    std::fs::create_dir_all(&resources).unwrap();
    std::fs::write(&launcher, "desktop launcher").unwrap();
    std::fs::write(resources.join("app.asar"), "desktop resources").unwrap();
    assert_eq!(cli_executable(&app), None);
    assert_eq!(cli_executable(&launcher), None);

    std::fs::write(&cli, "CLI").unwrap();
    let expected = Some(cli.canonicalize().unwrap());
    assert_eq!(cli_executable(&app), expected);
    assert_eq!(cli_executable(&launcher), expected);
    assert_eq!(cli_executable(&cli), expected);

    let nested = resources.join("codex-cli/CodexCLI.app/Contents/MacOS/codex");
    std::fs::create_dir_all(nested.parent().unwrap()).unwrap();
    std::fs::rename(&cli, &nested).unwrap();
    let expected = Some(nested.canonicalize().unwrap());
    assert_eq!(cli_executable(&app), expected);
    assert_eq!(cli_executable(&launcher), expected);
    assert_eq!(cli_executable(&nested), expected);
    assert_eq!(cli_executable(&root.join("missing")), None);
    std::fs::remove_dir_all(root).unwrap();
}
