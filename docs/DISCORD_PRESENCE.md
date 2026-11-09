# Discord Rich Presence research

Researched 2026-11-08. This is a proposed integration, not an implemented feature.

## Recommended approach

Use local Discord IPC with the community Rust crate `discord-rich-presence` (currently 1.1.0), subject to dependency/license and reconnect testing before adoption. It exposes DiscordIpcClient, connect, set_activity and close. This fits ReDox's native Rust architecture without adding an embedded web runtime. The official Social SDK is an alternative if broader Discord social features become necessary; the Rust IPC crate is community maintained, not an official Discord SDK.

Discord's official documentation permits unauthenticated Rich Presence via RPC. Desktop users need Discord running and a registered Discord Application ID. No user password, bot token, client secret, or OAuth login should be requested for this presence-only design. Discord browser clients do not support this direct RPC path.

## Setup and visible information

Create a ReDox application in the Discord Developer Portal and use its public Application ID. Upload a 1024 by 1024 ReDox logo under Rich Presence art assets; reference its lowercase asset key. Application registration and asset upload are still required before integration can be validated.

Suggested presence: ReDox as the application, details such as Coding / Generating response / Idle, a stable session start timestamp, and a GitHub repository button. Keep strings short and avoid duplicate information. Discord supports up to two URL buttons; these are visible to other users, not the account viewing its own presence.

Make presence opt-in. Project names and model names should have separate optional switches. Never include prompt text, chat titles, responses, paths, account details, token quotas or clipboard contents. Keep a generic activity available when project sharing is disabled.

## Implementation plan

Put transport and lifecycle in src/discord_presence.rs, with settings presentation in src/ui/. Persist the enable/share preferences with existing application settings. Use a single worker with a latest-state channel: IPC connection/read/write must not block egui. Connect when enabled, coalesce changes, and use bounded retry backoff when Discord is missing or restarted. Publish only on meaningful state changes, preserve the elapsed timestamp across updates, clear activity on disable, and close on shutdown. Presence errors must never stop a chat or consume its error/status display. Use a separate connection status in settings.

Verify against the selected crate's current API and Discord RPC limits before implementing throttling; do not send updates every UI frame. Do not implement a bot/self-bot or custom status automation.

## Verification before shipping

Cover enabled/disabled behavior, generic versus optional project/model data, unchanged-state deduplication, Discord absent at startup, Discord restart, malformed/disconnected IPC, worker shutdown, and failure isolation from chat generation. Live verification requires a real Application ID, uploaded art, running Discord, and another account to inspect buttons. Test Windows first, then supported macOS/Linux socket discovery.

## Primary sources

- Discord direct RPC and authentication requirements: https://docs.discord.com/developers/discord-social-sdk/development-guides/setting-rich-presence
- Discord design and artwork guidance: https://docs.discord.com/developers/rich-presence/best-practices
- Community Rust crate source and example: https://github.com/vionya/discord-rich-presence
- Current crate API: https://docs.rs/discord-rich-presence/latest/discord_rich_presence/
