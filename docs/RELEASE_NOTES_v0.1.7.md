ReDox v0.1.7 adds optional Discord Rich Presence with customizable text.

- Settings > About now includes Enable/Disable Presence and editable Details and State fields.
- Presence is disabled by default. Text and the enable preference persist across restarts.
- Background IPC keeps Discord connection work off the UI thread, coalesces text edits, and retries unavailable connections.
- Disabling clears presence; shutdown attempts to clear and disconnect with a bounded wait.
- Only custom presence text and elapsed session time are published; chat content and project paths are not included automatically.
- Includes the standalone presence-test target and the prior v0.1.6 chat queue and UI improvements.

Download ReDox-Windows-x64.exe. Existing v0.1.6 installations can discover this newer version through the updater.

Usage: open Discord desktop, start ReDox, then enable presence under Settings > About. Edit the two text fields to customize the profile activity.

Validation: cargo fmt --check, cargo check --bins, three Discord lifecycle/text tests and four About/settings tests. Optimized Windows build. Live Discord profile display has not been verified.
