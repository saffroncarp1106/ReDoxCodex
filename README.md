# CodexRustReWrite
A front-end rewrite for rust because 1.2g of ram for a basic ui window is ridiculous.


## Run

Double-click `Launch Codex Rust.cmd` to build and open this copy. Rust and the Visual Studio C++ build tools are required.

On macOS or Linux, with Rust and the native platform build dependencies installed, build and run with `cargo run --locked --bin codex-rust`. The `.cmd` launcher is for Windows.

Startup restores the saved window size. Saved sizes below the 900 × 600 minimum recover to 1100 × 900 before the window opens.

## Cursor animation

Open **Settings → Cursor animation** to add or edit names, select which names appear, choose a color palette, or customize individual colors. You can also adjust cursor count, size, duration, labels, trails, and guides, disable startup playback, or replay the animation. Choices are saved automatically for both layouts.

Release defaults are embedded from `src/ui/startup.json`: four size-15 cursors, a 5.5-second animation, the names SaffronCarp1106 and SackBoy19, and purple/pale blue colors (`#8C69B8`, `#ABD3D8`). Personal changes persist in `%APPDATA%\ForgeDesk\data\app.ron` under `codex-rust.ui.v1`; they are not temporary cache. **Restore animation defaults** returns to the bundled release settings. Existing personal choices take priority over release defaults.

Animation inspiration: [Swarm Rebuild by Dhvanil Patel](https://github.com/DhvanilPatel/swarm-cursors), which credits the [original Swarm animation by Joel Califa](https://califa.github.io/swarm-site/). This application uses a native Rust adaptation.

## Message formatting

Both layouts display PNG, JPEG, WebP, and animated GIF images from Markdown image tags or image-file links, including local paths, workspace-relative paths, embedded image data, and HTTP(S) URLs. Uploaded images are previewed in the conversation. Generated-image events are retained in live responses and imported history. Inline generated data persists under `%LOCALAPPDATA%\ForgeDesk\media` on Windows, `~/Library/Application Support/ForgeDesk/media` on macOS, and `$XDG_DATA_HOME/ForgeDesk/media` (default `~/.local/share/ForgeDesk/media`) on Linux. If Codex supplies no accessible image data or file, the message explains that instead of hiding the output.

Video links (MP4, WebM, MOV, MKV, AVI, M4V) show **Play video**, opening the file or URL in the default player/browser. Video playback is external; the app does not embed a video decoder. Viewing media does not request another model generation.

Both Codex and Gemini layouts now use the shared Markdown/math renderer in `src/ui/markdown.rs`. Equations, fractions, powers, subscripts, logarithms, headings, links, lists, and tables are formatted. Inline math accepts `$...$` or `\(...\)`; display math accepts `$$...$$` or `\[...\]`. Code blocks retain literal source and indentation, including HTML and LaTeX code examples. Right-click an equation to copy its LaTeX. Unsupported equations remain readable as source with an explanation on hover.


## Verification

```powershell
cargo fmt --check
cargo test --locked
cargo clippy --all-targets -- -D warnings
cargo build --locked
```


## v0.1.1 missing-file repair

The formatting preview is now bundled at `src/ui/previews/formatting.md`. Normal app builds no longer depend on a file under `tests/`. The referenced UI regression tests are included in `tests/ui/`; include that folder when distributing source for `cargo test` or `cargo fmt`.

Build with `cargo build --locked`, or double-click `Launch Codex Rust.cmd`. The source archive must include `src/`, `tests/`, `docs/`, `Cargo.toml`, `Cargo.lock`, `rustfmt.toml`, and the launcher. Generated `target/` and `temp/` directories are unnecessary for a source download.
