# UI packs

Codex Rust ships with `src/ui/layouts/codex.json` and `gemini.json`. Codex is the default. Packs change presentation; model requests still use Codex and your ChatGPT account.

Use either a single `src/ui/layouts/ocean.json` or a folder:

```text
src/ui/layouts/ocean/
  ui.json
  background.png
```

The filename or folder name is the pack ID. The `name` field is displayed in the dropdown. Avoid duplicate IDs. The menu rescans every three seconds and also provides a manual refresh. Invalid packs are listed under Settings → Appearance and become available once their configs and assets are valid.

Import a file or pack folder by dragging it into the app. Alternatively use UI → Custom… Import config and select its JSON file. Import copies the config and referenced image into a new pack folder. Existing packs are preserved with a numbered suffix when names collide. Import does not automatically activate a pack; select its name afterward.

## Example ui.json

```json
{
  "name": "Ocean",
  "layout": "gemini",
  "heading": "What are we building?",
  "placeholder": "Ask anything",
  "sidebar_default": false,
  "glow": "#224488",
  "glow_strength": 0.55,
  "palette": {
    "background": "#101418",
    "sidebar": "#151C23",
    "rail": "#101418",
    "surface": "#202A35",
    "text": "#E4EDF6",
    "menu_text": "#808080",
    "muted": "#A2B4C7",
    "border": "#344353",
    "accent": "#86BCFF"
  }
}
```

Optional: add `"background_image": "background.png"` and include that file beside the config. Subdirectories are supported. Images must be PNG, at most 8 MiB and 4096 × 4096 pixels. Asset references must stay within the pack folder. Background images are supported by both base layouts; the radial glow applies to Gemini's empty home screen.

`layout` accepts `codex` or `gemini`. Other fields are optional and inherit defaults. Colors use six-digit RGB hex strings. `glow_strength` ranges from 0 to 1. Labels are bounded, and configs must be at most 64 KiB. Unknown fields and invalid values produce an explicit error. Copy the complete built-in Gemini config when you want its exact palette as your starting point.

`codex.json` includes every adjustable palette key and a `palette_guide` object describing what each one controls. The guide is optional documentation metadata; edit the actual hex values under `palette`. `menu_text` controls the top menu labels independently of `text` (general text) and `muted` (secondary text and icons). Appearance color-picker choices override `menu_text` and `text` for that layout until **Restore text colors** is selected.

`input_background` controls the message input box independently of `surface`. When omitted, it inherits `surface` for compatibility with existing packs. The Appearance page also provides a **Text box background** picker, saved separately for each layout; **Restore text colors** clears that override along with the text overrides.

`sidebar_default` is used on first selection. Later choices are remembered separately for each pack. UI selection, chats, drafts, workspace, and model/effort choices survive restart. A missing selected pack falls back to Codex. The cursor animation reads actual rendered geometry from either layout. Customize it in Settings → Cursor animation; saved choices apply to both layouts. `src/ui/startup.json` supplies the initial defaults before personal overrides are saved.

Choose UI → Reload current UI after editing the active config. Selecting or reloading a pack saves the session and restarts the process. Switching is disabled while a response, sign-in, or import is running.

For portable builds, put packs under `ui/layouts/` beside the executable. `CODEX_RUST_UI_FOLDER` can point to another pack directory. Source checkouts discover `src/ui/layouts/` relative to the executable, so the launcher works from any current directory. Built-in Codex/Gemini configs remain embedded as fallbacks when external files are absent.

## Verification

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo build --locked
.\target\debug\codex-rust.exe --screenshot temp\screenshots\gemini-home.png --ui=gemini --home-preview --startup-preview=4.2
```

Create the screenshots folder first. Capture mode does not save changes to your session or UI preferences. `--home-preview` opens an empty chat in memory for layout verification.

Debug builds also support an isolated restart check: set `CODEX_RUST_VERIFY_UI_RESTART` to a new empty directory and launch the executable. It uses a separate `state.ron`, switches from Codex to Gemini, checks draft persistence after the animation, writes `result.json`, and closes. This does not send model prompts. Clear the variable before ordinary launches.

For structural changes beyond the two base layouts, edit the native presentation modules described in [UI_CUSTOMIZATION.md](UI_CUSTOMIZATION.md) and rebuild.
