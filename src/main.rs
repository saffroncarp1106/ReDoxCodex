mod app;

use app::ForgeDesk;
use eframe::egui;

fn main() -> eframe::Result {
    let args: Vec<_> = std::env::args_os().collect();
    let capture_path = if args.get(1).is_some_and(|arg| arg == "--screenshot") {
        args.get(2).map(std::path::PathBuf::from)
    } else {
        None
    };
    let size = if capture_path.is_some() {
        [956.0, 900.0]
    } else {
        [1100.0, 900.0]
    };
    let mut viewport = egui::ViewportBuilder::default()
        .with_app_id("ForgeDesk")
        .with_decorations(false)
        .with_drag_and_drop(true)
        .with_resizable(true)
        .with_inner_size(size)
        .with_min_inner_size([900.0, 600.0]);
    #[cfg(target_os = "macos")]
    {
        viewport = viewport
            .with_decorations(true)
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false)
            .with_titlebar_buttons_shown(false);
    }
    if capture_path.is_some() {
        viewport = viewport.with_maximized(false).with_position([80.0, 60.0]);
    }
    let options = eframe::NativeOptions {
        window_builder: Some(Box::new(ForgeDesk::validate_window_size)),
        persist_window: capture_path.is_none(),
        persistence_path: if cfg!(debug_assertions) {
            std::env::var_os("CODEX_RUST_VERIFY_UI_RESTART")
                .map(|path| std::path::PathBuf::from(path).join("state.ron"))
        } else {
            None
        },
        viewport,
        ..Default::default()
    };

    let restart = std::sync::Arc::new(std::sync::Mutex::new(false));
    let app_restart = restart.clone();
    eframe::run_native(
        "Codex Rust",
        options,
        Box::new(move |context| Ok(Box::new(ForgeDesk::new(context, capture_path, app_restart)))),
    )?;
    if restart.lock().is_ok_and(|restart| *restart) {
        match std::env::current_exe().and_then(|path| std::process::Command::new(path).spawn()) {
            Ok(_) => {}
            Err(error) => eprintln!(
                "UI saved, but could not restart Codex Rust: {error}. Reopen the launcher."
            ),
        }
    }
    Ok(())
}
