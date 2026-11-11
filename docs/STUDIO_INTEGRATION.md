# Studio and ReDox integration

Both applications consume Studio's typed v2 `.redoxskin` document and its scene renderer. ReDox v0.2.1 bundles the required Studio library under `src/ui/studio_runtime`. Ordinary ReDox builds work independently of the sibling ReCreation project. Presentation stays under `src/ui/`, and host dispatch lives in `src/ui_actions.rs`. Studio authoring and source scene exports use the separate ReCreation project.

## Try it

1. Build both applications with their existing launchers after closing older builds.
2. In Studio's Assets library, open **Wii live workspace** for a complete conversation/composer example, or choose **Wii functional controls** / **Wii asset controls** for the larger Wii reconstruction. MCP-created copies are in `ReCreation/UI TEST`.
3. Select a layer. Choose its **ReDox UI script** in the inspector. Move, resize, duplicate, group or restyle it as usual. Replace an image with **Replace layer image (PNG)**, or drop a PNG onto an editable layer on the canvas. IDs and bindings survive image replacement; undo restores the prior document.
4. Save a `.redoxskin` copy. In ReDox, open Settings → Appearance → **Load Studio controls…** and choose that file, or drag the file onto ReDox.
5. Use the controls window alongside any built-in theme. Subsequent valid Studio saves reload automatically. **Reload from Studio** also forces a refresh. ReDox remembers the selected file path.

The first page is the runtime surface. Studio's Play mode and detached simulated preview run dummy messages, native-style menus and model/effort choices locally. Redox binds these controls to the real session, active chat transcript and existing host actions.

## UI script contract

A layer can be decorative (no calls), a clickable action, a native control, or a short sequence of host calls:

```json
"ui_script": ["history", "chats"]
```

The full layer retains the normal Node geometry/style fields. These calls are typed, ordered and limited to 16. Unknown calls fail validation. A native control must be the only call on its layer. This is the first declarative scripting contract; arbitrary Rust, JavaScript, shell code, conditions and user-defined functions are not implemented.

| Call | Runtime behavior |
| --- | --- |
| `history` | Show sidebar and refresh history |
| `chats` | Show chats sidebar |
| `project`, `project.open` | Open workspace chooser |
| `chat.new` | Start a chat through existing availability checks |
| `chat.send`, `chat.stop` | Existing send/queue and cancellation flows |
| `chat.previous`, `chat.next` | Navigate chats |
| `chat.rename` | Rename current chat |
| `ollama.agent` | Open Agents panel |
| `settings` | Open Settings |
| `sidebar.toggle` | Toggle sidebar |
| `files.add` | Existing attachment chooser |
| `menus` | Native application menus |
| `model`, `effort` | Native model and supported-effort pickers |
| `model.effort` | Combined model and supported-effort popup |
| `composer` | Edit the active draft; Enter sends, Shift+Enter adds a newline |
| `conversation` | Scroll the active chat transcript inside the authored panel |
| `settings.account` … `settings.about`, `settings.close` | Navigate or close the corresponding Settings page |

Bindings are explicit; a label that says “Send” has no behavior until bound. Hidden ancestors and parent clipping constrain hit regions. Overlapping bound regions dispatch the topmost region once. Core new-chat/project/attachment/send/stop/navigation buttons in existing layouts use the same host dispatcher.

Files without `ui_script` retain their old appearance and behavior. Rebuild both apps before sharing files with these new fields: older strict v2 readers do not know this extension. Embedded PNGs are portable; replacing an image compacts unused textures and retains the existing document limits. The asset example includes reusable gradient surface textures and separate editable text.

## MCP authoring

`studio_list_originals` exposes `wii-controls`, `asset-controls` and `wii-live-workspace`, plus fifty source scene templates. Use a catalog ID as `template_id` in `studio_create_document`. Operations accept `properties.ui_script` on creation and `patch.ui_script` on updates. Set `[]` to clear bindings. Inspection, validation, save, history and file-conflict rules apply.

```json
{
  "op": "create_node",
  "ref": "historyshortcut",
  "page_id": "RETURNED_PAGE_HANDLE",
  "kind": "Button",
  "properties": {
    "name": "History shortcut",
    "text": "History",
    "ui_script": ["history"]
  }
}
```

## Next implementation stages

This release adds a functional controls window with an active conversation slot. The main application shell still supplies mandatory controls. Further shell integration needs:

1. Add typed dynamic slots for history rows, projects, queues, account status and approvals. The `conversation` slot already displays the active chat.
2. Add responsive constraints and minimum sizes so layouts adapt beyond a fixed design surface.
3. Add a required-capability validator and built-in fallback controls. Mandatory controls currently remain available in the existing Redox shell.
4. Expand the action catalog to context-specific commands. Queue IDs, archive/delete targets, approvals and Settings fields need typed arguments.
5. Add a versioned scripting language with parameters and conditions, plus a Studio “Run in Redox” bridge. Saved controls already hot-reload; MCP asset upload and direct editing of unsaved native documents remain follow-up work.

## Verification

Studio library tests exercise old document compatibility, script validation, image replacement, round trips and real egui click dispatch. Studio's native state tests and ReDox's binary tests also run without observing or controlling the desktop. The standalone MCP server was used to create, bind, validate, render and save the supplied example. Full `cargo test` in Studio can fail if Windows has its normal executable open; `cargo test --lib` and `cargo test --bin redox-studio` avoid replacing that executable.

## Live customizer update

Studio's **Play** mode and detached Live preview now provide a local dummy-chat session. The scene library contains welcome, conversation and every Settings page for all five themes. Scene templates use IDs such as `win98:about` in MCP. Source glyphs are preserved until edited, and Redox watches saved Studio controls for changes. See [Live customizer](../../ReCreation/docs/LIVE_CUSTOMIZER.md) for the workflow and simulation scope.
