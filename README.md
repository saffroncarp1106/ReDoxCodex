# CodexRustReWrite
A front-end rewrite for rust because 1.2g of ram for a basic ui window is ridiculous.

Redox: A reaction where both reduction and oxidation occurs. The process of Oxidation, is the lose of an electron, the project was aimed to rewrite Codex to no longer use Electron and instead Rust, ironically when Iron oxidizes, it Rusts. Technically a redox reaction needs a reduction, which we have by reducing ram consumption by 10x, so use the free memory for an Ollama model!

## Run
Run the application after building using the command below, and open `ReDoxCodex\target\debug\codex-rust.exe`

When making edits `Launch Codex Rust.cmd` to build and open this copy. Rust and the Visual Studio C++ build tools are required.

On macOS or Linux, with Rust and the native platform build dependencies installed, build and run with `cargo run --locked --bin codex-rust`. The `.cmd` launcher is for Windows.

Startup restores the saved window size. Saved sizes below the 900 × 600 minimum recover to 1100 × 900 before the window opens.

Chat rows have a small **×** button to archive locally. Open **Archived chats** and use **↶** to restore them. These choices persist across restarts and history refreshes without changing the server's archive state. Chats generating a response must finish or stop before archiving.

**New chat** reuses an empty conversation for the same project and provider, and is disabled while that blank chat is selected. Creating or returning to a blank chat focuses the message box in both layouts. Drafts and attachments are preserved when starting another chat.

## Desktop control (Windows)

Enable **Settings → Workspace & access → Allow Codex to see and control this desktop**. Codex then receives the native `desktop_control` tool for screenshots, window discovery/focus, clicking, dragging, scrolling, Unicode typing, and key chords. Existing chats pick up the tools on their next response while keeping the visible transcript. No desktop skill, Electron runtime, extra desktop server, or API key is required.

Desktop access is off by default and saved across restarts. Screenshots of all monitors are sent to your Codex model as images; they are captured on demand, without recording video or saving captures to disk. Input asks for approval once per response by default; this can be changed beside the toggle. After approval, Codex refreshes its view before acting. **Stop** cancels further actions; holding **Escape** blocks desktop actions. Desktop access is separate from project file permissions and is available to Codex, not Ollama workers. Windows can refuse focus or input to elevated apps, the lock screen, or a disconnected session. Screenshots include their physical pixel dimensions and desktop origin for multiple monitors. Each response is capped at 100 desktop calls.

If a saved project folder was moved or renamed, sending a request opens Workspace settings instead of attempting a command in that invalid folder (Windows error 267). Choose the new project location to reconnect that chat and preserve the draft. You can also select **Use selected project for this chat** when switching an existing conversation to the currently selected project.

macOS and Linux retain chat/project support; this desktop backend is Windows only. Tool integration follows the [Codex app-server dynamic tool protocol](https://learn.chatgpt.com/docs/app-server).

## Ollama agents

Open the **Agents** tab on the right, connect to your Ollama server (default `http://127.0.0.1:11434`), and choose an installed model such as Qwen. **Use for this chat** hands the conversation to that model; **New agent chat** starts a separate local chat. **Switch this chat to Codex** returns to the account provider. New chats inherit the active provider and local model. The server, preferred model, agent instructions, per-chat provider, and token usage persist across restarts.

To keep Codex in charge, enable **Let Codex delegate to this agent**. Codex receives a native `delegate_to_ollama` tool and can command the selected worker using the current project and access mode. Ask for delegation in the main chat, or type a task in Agents and click **Ask Codex to delegate**. Progress and worker output appear at the top of Agents, and the worker result returns to Codex for review and the final response. Task logs survive restart; Stop cancels the parent run and its worker. File changes still use the existing approval flow. Existing Codex conversations without the tool start a fresh server thread while retaining the visible transcript. Each Codex turn is limited to eight delegations. Ollama worker tokens are counted in the panel; Codex orchestration and review still use Codex quota.

Ollama responses stream directly into the normal conversation in either layout. The composer identifies the local model, and the Agents panel shows input/output tokens, request counts, and generation speed. Local Ollama calls do not consume Codex account quota; a remote or cloud-backed Ollama model may have its own provider usage. No models are installed or downloaded by the application.

With **Allow project tools** enabled, models advertising tool support can list/read project files, create directories, write files, and run commands. Choose a project folder and the existing access mode. Read-only mode exposes only file inspection. Workspace mode confines file tools to the project and asks for approval before changes or commands. Full access permits external file paths and skips approvals. Shell commands run with your OS account permissions, without an OS sandbox; they are limited to 30 seconds and bounded output. Replaced files are preserved in the platform's local application-data folder under `ForgeDesk/agent-backups`. The local agent loop stops after 16 model steps. Models without tool support still work as chat models. Image attachments require a model advertising vision support.

The integration uses Ollama's native [model list](https://docs.ollama.com/api/tags), [streaming chat API](https://docs.ollama.com/api/chat), and [tool calling](https://docs.ollama.com/capabilities/tool-calling). Ollama transport and project actions are in `src/ollama.rs` and `src/ollama_tools.rs`; the panel is in `src/ui/agents.rs`.

If Ollama reports a CUDA/GPU initialization failure, enable **CPU only (GPU troubleshooting)** in Agents and retry. This sets `num_gpu: 0` for the request; CPU generation can be slower.

## Model and effort

Both composers use a combined model/effort control. Click it to open a compact card with the current effort, model selector, reset button, and stepped effort slider. Mouse and Left/Right keys select only levels supported by that model; choices remain saved per model. Reset restores the server's model default. Ollama chats show the local model selector without Codex reasoning levels.

The card is centered over its opening button, with a below-button fallback when needed. Clicking the model name expands a scrollable list inside the card; choosing a model collapses that list while keeping the effort card open.

## Text colors

Open **Settings → Appearance → Text colors** to adjust **Top row text** (File/Edit/View/Help and the UI selector) independently of **General text** (the main interface and conversations). Color pickers apply live and save separately for each layout. **Restore text colors** returns to the layout defaults. UI pack JSON can also set `palette.menu_text` separately from `palette.text`; existing packs without this field use the softer gray menu default.

The complete adjustable Codex palette is explicit in `src/ui/layouts/codex.json`. Its `palette_guide` explains each key next to the editable `palette`. Change `menu_text` for the top row, `text` for general text, and `muted` for secondary text/icons. Saved Appearance choices override `text` and `menu_text`; use **Restore text colors** to return control to the JSON. Reopen the app or choose **UI → Reload current UI** after file edits.

**Text box background** on the same Appearance page controls the input box independently of other surfaces in both layouts. Its JSON key is `palette.input_background`; packs without this key inherit `surface`. Appearance choices for this color are saved per layout too. The Codex header's robot button toggles the Ollama Agents panel, and the sidebar toggle sits to the left of the chat title. The access label and shield use orange only for Full access; Read only and Workspace access use general text.

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
