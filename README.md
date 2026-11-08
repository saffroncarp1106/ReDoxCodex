# ReDoxCodex
A front-end rewrite for rust because 1.2g of ram for a basic ui window is ridiculous.

Redox: A reaction where both reduction and oxidation occurs. The process of Oxidation, is the lose of an electron, the project was aimed to rewrite Codex to no longer use Electron and instead Rust, ironically when Iron oxidizes, it Rusts. Technically a redox reaction needs a reduction, which we have by reducing ram consumption by 10x, so use the free memory for an Ollama model!

## Run
On Windows, open `Launch ReDox.cmd` to rebuild changed project files and then start the updated release executable. Cargo skips unchanged work. Use `Launch ReDox.cmd --debug` for a debug build. A failed build stops the launcher instead of starting an older executable. This uses local source files and does not download source updates. Rust and the Visual Studio C++ build tools are required.

On macOS or Linux, with Rust and the native platform build dependencies installed, run `cargo run --locked --bin redox`.

After you discover **Windows 98**, choose it from the **UI** menu for the teal desktop, classic chat window, and Start taskbar. **My Chats**, **Open Project**, and **Ollama Agents** open their corresponding ReDox tools.

Startup restores the saved window size. Saved sizes below the 900 × 600 minimum recover to 1100 × 900 before the window opens.

Chat rows have a small **×** button to archive locally. Open **Archived chats** and use **↶** to restore them. These choices persist across restarts and history refreshes without changing the server's archive state. Chats generating a response must finish or stop before archiving.

**New chat** reuses an empty conversation for the same project and provider, and is disabled while that blank chat is selected. Creating or returning to a blank chat focuses the message box in every layout. Drafts and attachments are preserved when starting another chat.

## Desktop control (Windows)

Enable **Settings → Workspace & access → Allow Codex to see and control this desktop**. Codex then receives the native `desktop_control` tool for screenshots, window discovery/focus, clicking, dragging, scrolling, Unicode typing, and key chords. Existing chats pick up the tools on their next response while keeping the visible transcript. No desktop skill, Electron runtime, extra desktop server, or API key is required.

Desktop access is off by default and saved across restarts. Screenshots of all monitors are sent to your Codex model as images; they are captured on demand, without recording video or saving captures to disk. Input asks for approval once per response by default; this can be changed beside the toggle. After approval, Codex refreshes its view before acting. **Stop** cancels further actions; holding **Escape** blocks desktop actions. Desktop access is separate from project file permissions and is available to Codex, not Ollama workers. Windows can refuse focus or input to elevated apps, the lock screen, or a disconnected session. Screenshots include their physical pixel dimensions and desktop origin for multiple monitors. Each response is capped at 100 desktop calls.

If a saved project folder was moved or renamed, sending a request opens Workspace settings instead of attempting a command in that invalid folder (Windows error 267). Choose the new project location to reconnect that chat and preserve the draft. You can also select **Use selected project for this chat** when switching an existing conversation to the currently selected project.

macOS and Linux retain chat/project support; this desktop backend is Windows only. Tool integration follows the [Codex app-server dynamic tool protocol](https://learn.chatgpt.com/docs/app-server).

## Future Goals

Update to another Repo for custom UI's, with a detailed menu on how to make that
To Do:

## Ollama agents

Open **Agents** on the right, connect to your Ollama server (default `http://127.0.0.1:11434`), and choose an installed model such as Qwen. **Use for this chat** hands the conversation to that model. **New agent chat** starts a separate local chat, and **Switch to Codex** returns to the account provider. New chats inherit the active provider and local model. The server, preferred model, agent instructions, per-chat provider, and token usage persist across restarts.

To keep Codex in charge, enable **Let Codex delegate to Ollama**. Codex receives a native `delegate_to_ollama` tool and can command the selected worker using the current project and access mode. Ask for delegation in the main chat, or type a task in Agents and click **Ask Codex to delegate**. Progress appears at the top of Agents. Expand a task to see its output. The worker result returns to Codex for review and the final response. Task logs survive restart. Stop cancels the parent run and its worker. File changes still use the existing approval flow. Existing Codex conversations without the tool start a fresh server thread while retaining the visible transcript. Each Codex turn is limited to eight delegations. Ollama worker tokens are counted in the panel. Codex orchestration and review still use Codex quota.

Ollama responses stream directly into the normal conversation in either layout. The composer identifies the local model, and the Agents panel shows input/output tokens, request counts, and generation speed. Local Ollama calls do not consume Codex account quota; a remote or cloud-backed Ollama model may have its own provider usage. No models are installed or downloaded by the application.

With **Allow project tools** enabled, models advertising tool support can list/read project files, create directories, write files, and run commands. Choose a project folder and the existing access mode. Read-only mode exposes only file inspection. Workspace mode confines file tools to the project and asks for approval before changes or commands. Full access permits external file paths and skips approvals. Shell commands run with your OS account permissions, without an OS sandbox; they are limited to 30 seconds and bounded output. Replaced files are preserved in the platform's local application-data folder under `ReDox/agent-backups`. Models without tool support still work as chat models. Image attachments require a model advertising vision support.

Local agents have no fixed model-step or per-reply tool-call count limit. ReDox retries a failed Ollama request up to three times after connection drops, timeouts, temporary server errors, or incomplete responses. Each retry resumes at the failed model step using the tool results already recorded. The transcript keeps partial text until a retry replaces it, and **Stop** cancels pending retries. Persistent failures end the turn with the error. A response cut short by the output token limit continues automatically. Tool calls from incomplete responses are discarded.

ReDox reuses the selected context window (including 32k) by summarizing progress and starting a fresh message window before the growing transcript fills it. If summarization still fails after retries, ReDox continues with an excerpt from the saved checkpoint. Generation reserves at least three quarters of the context for input and overhead, so the effective response token allowance can be lower than the configured output maximum. Oversized individual tasks, instructions or images must still fit the selected window.

Each local chat keeps a `steps.md` checkpoint at `%LOCALAPPDATA%\ReDox\agent-state\<chat-id>\steps.md` on Windows, or the platform's local application-data folder under `ReDox/agent-state` elsewhere. It records the task, assistant progress, requested tools, actual results, completion status, and a model-written summary with remaining work at context rollover. The application saves before actions and after results, and loads that chat's checkpoint on the next request, including after restart. To resume interrupted work, reopen the same local chat and send **Continue**; the agent is told to inspect any action without a recorded result before repeating it. Different chats and delegated tasks have separate checkpoints; delegated tasks continue within their current invocation, and a new delegation starts a new checkpoint. Checkpoints are application state, so read-only project access remains read-only and a project's own `steps.md` is untouched. The existing desktop-action cap and project approval rules still apply. Checkpoint summarization adds local model requests and appears in usage counts.

This continuation path is covered by mock Ollama transport tests and can be built without Ollama installed. Actual model behavior, including `qwen3.5-abliterated:9b`, requires testing against your Ollama server; ReDox does not force a model to call tools or guarantee the quality of its summaries.

The integration uses Ollama's native [model list](https://docs.ollama.com/api/tags), [streaming chat API](https://docs.ollama.com/api/chat), and [tool calling](https://docs.ollama.com/capabilities/tool-calling). Ollama transport, progress checkpoints, and project actions are in `src/ollama.rs`, `src/ollama_state.rs`, and `src/ollama_tools.rs`; the panel is in `src/ui/agents.rs`.

If Ollama reports a CUDA/GPU initialization failure, open **Model & tool options** in Agents, enable **CPU only (GPU troubleshooting)**, and retry. This sets `num_gpu: 0` for the request. CPU generation can be slower.

## Cursor animation

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


ReDox checks for updates at startup and offers **Yes** (download and restart) or
**No** when a newer release is available. **Never show again** is saved and can
be changed in **Settings ? Updates**, where manual checks and **Update and restart**
remain available. Private releases require `REDOX_UPDATE_TOKEN`.

On Windows, updates download the release asset named `ReDox-Windows-x64.exe`
and replace the running executable in its current folder without rebuilding.
The generated helper is `<executable folder>/temp/update-<id>/update.ps1`.
It waits for ReDox to exit, preserves `redox-previous.exe` there, replaces the
executable, and restarts it. Replacement failures restore the backup; helper
errors are recorded in `update-error.txt` in the same directory. The installation
folder must be writable. Chats and settings remain in application storage.
Updates wait until the current response finishes before closing the application.
Other platforms can use **View release and download**.

The launcher uses existing executables; run `Build Release.bat` explicitly after
source changes. Release tags must use `vMAJOR.MINOR.PATCH` or `MAJOR.MINOR.PATCH`.
A test tag must include the compiled Windows asset to support installation.

The default update repository is `saffroncarp1106/ReDoxCodex`.
`REDOX_UPDATE_REPOSITORY` optionally overrides it. For private releases, set
`REDOX_UPDATE_TOKEN` in the environment to a GitHub token with read access to
that repository. Each user needs their own access; tokens are not bundled or
saved in application settings. Git credential-manager access alone does not
authenticate the updater's GitHub API request.

Replying to a conversation imported from Codex history creates a separate ReDox chat with the loaded conversation context. The original history remains separate.

Loaded Codex history chats also appear under their project folders. History rows include the project folder name; use Load more chats to discover older conversations.
