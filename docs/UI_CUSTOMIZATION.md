# Customizing the interface

The entire presentation layer is in `src/ui/`. Codex transport, authentication, file attachments, persistence, and execution stay in the application/backend modules.

| File | Responsibility |
| --- | --- |
| `mod.rs` | UI entry point, settings sections, persisted appearance preferences |
| `packs.rs`, `layouts/` | Pack discovery, config validation, asset import, and built-in configs |
| `picker.rs` | UI dropdown and drop target |
| `gemini.rs` | Gemini-style home, rail, composer, and conversation layout |
| `shell.rs` | Window chrome, navigation rail, projects, local chats, shared Codex history and pagination controls |
| `conversation.rs` | Conversation layout, messages, Markdown/code blocks, composer, attachment chips |
| `markdown.rs`, `media.rs` | Shared Markdown/math, inline images and animated GIFs, video playback links |
| `controls.rs` | Model and effort display helpers and effort control |
| `settings.rs` | Account, model/reasoning, workspace/access, appearance, and advanced settings |
| `usage.rs` | Usage popup anchored to the bottom-left reticle |
| `dialogs.rs` | Rename and approval dialogs |
| `icons.rs` | Vector icon artwork and icon buttons |
| `theme.rs` | Color tokens, typography, default spacing, and widget styling |
| `startup.rs`, `startup.json` | Native cursor swarm startup choreography and editable configuration |
| `startup_settings.rs` | Cursor name selection, palette presets, color editing, replay, and source credits |
| `capture.rs` | Native screenshot verification utility |
| `window_frame.rs` | Native title-bar dragging, maximize/restore, edge and corner resizing |

## Config-based UI packs

Start with [UI_PACKS.md](UI_PACKS.md) to add configs, choose a base layout, edit colors/copy, or supply a PNG background without rebuilding. Import via the UI dropdown or drag a config/folder into the window. Packs in src/ui/layouts/ are discovered automatically. Theme color functions use the active palette, including Settings. Restart occurs after eframe has saved and closed the current window; the new process replays startup against the selected layout.

## Make a Rust UI variant

1. Edit colors, fonts, and shared spacing in `theme.rs`.
2. Change icon artwork in `icons.rs`.
3. Edit the relevant layout file for structural changes.
4. Keep `ForgeDesk::draw_ui` as the presentation entry point, or replace its calls with your own views.
5. Run `cargo fmt`, `cargo check`, then `cargo run`.

These are native Rust UI modules. Source changes require rebuilding. Authentication and model transport do not need to change for a new layout or theme.

`app.rs` binds this folder as its `ui` child module, so views can use application state and call existing actions without exposing backend internals publicly. Keep network, process, and filesystem operations in application actions. Add new controls only when their actions work.

## User preferences

Interface scale and sidebar visibility are stored separately from conversation data under `codex-rust.ui.v1`. Account credentials are managed by Codex and are not stored in UI preferences. New appearance preferences belong in `Preferences` in `mod.rs`.

**Settings → Cursor animation** manages the startup animation in either layout. Add, edit, remove, and select names; choose Classic, Pastel, Ocean, or Sunset palettes; or edit swatches and add colors for a custom palette. Selected names and colors cycle across the configured cursors. At least one nonblank name and valid color are retained. Replay applies changes immediately; startup playback can be disabled separately.

Animation overrides are saved in the `startup` field of `codex-rust.ui.v1`. Before overrides exist, `startup.json` (or `CODEX_RUST_STARTUP_CONFIG`) supplies the initial configuration. Restore animation defaults loads the bundled defaults. Credits link to [Dhvanil Patel's Swarm Rebuild](https://github.com/DhvanilPatel/swarm-cursors) and [Joel Califa's original Swarm](https://califa.github.io/swarm-site/).

## Verify a layout

Build first, then capture the app's own renderer:

```powershell
cargo build --locked
.\target\debug\codex-rust.exe --screenshot temp\screenshots\chat.png
.\target\debug\codex-rust.exe --screenshot temp\screenshots\account.png --settings=account
.\target\debug\codex-rust.exe --screenshot temp\screenshots\models.png --settings=models
.\target\debug\codex-rust.exe --screenshot temp\screenshots\workspace.png --settings=workspace
.\target\debug\codex-rust.exe --screenshot temp\screenshots\appearance.png --settings=appearance
.\target\debug\codex-rust.exe --screenshot temp\screenshots\advanced.png --settings=advanced
.\target\debug\codex-rust.exe --screenshot temp\screenshots\usage.png --usage
```

To capture the startup choreography at a fixed time (in seconds), use:

```powershell
.\target\debug\codex-rust.exe --screenshot temp\screenshots\startup.png --startup-preview=1.8
```

Without the preview flag, screenshot mode waits for the startup animation to finish. Do not use the preview flag for normal launches: it freezes the animation for visual verification.

To verify media without a model call, create local image/GIF/video fixtures and a Markdown file linking them, then run `target/debug/codex-rust.exe --screenshot temp/screenshots/media.png --media-preview=temp/media-fixtures/preview.md`. Paths in that Markdown resolve relative to the preview file. Add `--ui=gemini` to check the other layout. This preview is only applied in screenshot mode and does not save the temporary chat.

Create `temp/screenshots/` before capturing. Screenshot mode uses the current local account and conversation data, waits for connection discovery and history loading, saves one frame, and closes. Keep captures in `temp/` and review their content before sharing.

Use the project checks before contributing changes:

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
