# Progress

## Downloaded-copy formatting repair

This copy now connects src/ui/markdown.rs to both user and assistant messages in both layouts. Markdown and LaTeX equations render normally; code and HTML examples remain literal inside code blocks. Missing referenced UI test files and the formatting fixture were restored under tests/. For this downloaded copy, 14 tests, formatting, Clippy, and the locked build passed. The native pH formatting screenshot was inspected at temp/screenshots/formatting-fixed.png. Earlier verification notes below describe the upstream project history.

## Current implementation

Native Rust Codex/ChatGPT client using the official Codex app-server. Current launcher: Launch Codex Rust.cmd. Current executable: target/debug/codex-rust.exe.

- Switchable Codex/Gemini UI packs, title-bar/Appearance dropdown, drag/drop import, automatic discovery, configurable palette/PNG assets, and real process restart with preserved session and replayed animation.
- Codex-style native UI, projects, persistent messages/drafts, streamed replies, model picker, account settings.
- Individual text/code files and supported images via + > Add files. Text snapshots are sent with the prompt; attachments remain visible on messages.
- Read-only workspace, Workspace access, and Full access. Full access uses danger-full-access with approvalPolicy never.
- Old no-tools conversations migrate to fresh server sessions while retaining local history and previous server IDs.
- Mode changes restart the server conversation with the updated permissions and retained visible history.
- Desktop Codex.exe paths are corrected to the actual bundled CLI; current local CLI installations are also discovered.
- Device sign-in produces a typed challenge displayed as a selectable verification code, Copy code button, and server-provided link. Successful sign-in refreshes account/models automatically.
- Existing sign-in is reused. No credentials are copied or logged.
- Per-model reasoning effort picker in the composer and Settings; supported levels and defaults come from model/list, selections persist per model, and the chosen effort is sent with turn/start.
- Usage limits provide real account quota windows, usage/remaining percentages, reset countdowns, optional credits and reset counts, refresh, and explicit unavailable-data states.
- Usage limits now open from the bottom-left reticle, anchored beside it; the bell/profile open account settings.
- Presentation is separated under src/ui/ into shell, conversation, controls, settings, usage, dialogs, icons, theme, and capture modules. docs/UI_CUSTOMIZATION.md explains UI variants.
- Categorized settings include working account, model/effort, project/access, scale/sidebar, CLI selection, reconnect, and diagnostic-copy controls. Appearance preferences persist independently of chat data.
- Shared local Codex history loads through thread/list and thread/turns/list, with title search, archived chats, pagination, and continuing the original thread. Opening history does not start inference. Drafts survive reloads; older pages prepend without duplicating chat records.
- Configurable native cursor swarm startup reveals the live interface, then exits and fades away. src/ui/startup.json controls timing, count, size, names, colors, guides, trails, and worker assignment. Build targets are actual rendered boxes, text, and icons, with containing boxes completed before their contents. Escape/Skip bypasses it; no model requests are made by the animation.
- Native title-bar drag/maximize behavior and edge/corner resize handling are in src/ui/window_frame.rs, including drag-command and resize hit-testing checks.

## Verified

Cargo check, formatting, Clippy, build, and 47 automated tests pass. UI pack tests cover fallback, discovery, asset validation, import without replacement, and saved per-UI preferences. An isolated native restart check switched Codex to Gemini, preserved its draft, and completed the replayed animation. Startup tests cover bounded/partial configuration, exact rendered box geometry and corner radii, offscreen clipping, parent-before-text dependencies, automatic completion, and Escape-to-skip. Native screenshots verify the assembly and completed interface. The read-only live history test also passed for listing, pagination, archived history, title search, and transcript loading. The live device-sign-in test returned a real verification code and preserved the existing account. Earlier live model tests verified streaming and server conversation resume. File and attachment tests cover snapshots, file limits, workspace inheritance, and migration of old restricted sessions. The live effort/usage test retrieved actual account limits and received a model response with an explicitly selected, supported reasoning effort.

The live file-access regression test also passed: a migrated conversation used a real file-reading command, an attached snapshot was returned after its disk file changed, and Full access read a separate temporary file outside the workspace. Test file contents were random values absent from the prompt.

## Limits

Independent client, not an upstream desktop-source fork. It does not import chatgpt.com website history. Completing browser authentication is left to the user. General server requests beyond command/file approvals are explicitly rejected. Non-text documents such as PDFs are not yet extracted.

## File organization

Active files stay in src/, tests/, and docs/. Superseded scripts, handoffs, schema dumps, backups, and old binaries are preserved in temp/. See temp/MOVED_FILES.md. Cargo's active cache remains in target/.

