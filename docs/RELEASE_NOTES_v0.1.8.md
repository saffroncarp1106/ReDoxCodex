ReDox v0.1.8 adds account logout controls.

- Settings > Account now includes Log out for the Codex account shared with the CLI.
- Logout runs in the background and clears account, model, and quota details after success.
- Saved chats and drafts are preserved; errors are displayed without clearing the account.
- Logout is unavailable during a response, sign-in, or account refresh.
- Help links to the ReDox GitHub repository, with a small sidebar spacing adjustment.

Download ReDox-Windows-x64.exe, or use Settings > Updates > Update and restart from an earlier release.

Live account logout and visual appearance have not been verified.

Validation: cargo fmt --check and cargo test --locked passed. Optimized Windows build.
