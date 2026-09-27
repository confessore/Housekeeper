# Housekeeper

Housekeeper is a modular Rust Discord bot for inhouse communities. One bot instance can serve every Discord guild it has been invited to; configuration and role mappings are isolated per guild.

## Current commands

- `/housekeeper` — Open a private button-based hub for `/whoishere`, lobby list/current/create actions, Steam linking, and help. Shared lobby and voice panels are posted once per channel/view and reused when possible; slash commands remain available as an alternative.
- `/help` — explain how to use every Housekeeper command, including required permissions and guild scope.
- `/whoishere [balance]` — list non-bot players in your current voice channel, ordered by rank. Each linked player includes OpenDota, Dotabuff, and Stratz profile links. This is an ad-hoc voice scan, not the official lobby.
- `/lobby-create [hours]`, `/lobby-list`, `/lobby-close <number>` — create, inspect, or close numbered 10-player lobbies. Numbers are assigned automatically, reuse the smallest available number, and lobbies expire within 24 hours.
- `/lobby-join <number>`, `/lobby-leave` — join or leave a numbered lobby. A player can be in only one lobby per server, and a linked Steam account is required. Rank and win/loss data are refreshed when joining, with cached data used if OpenDota is unavailable.
- `/lobby-add <number> <user>`, `/lobby-remove <user>` — moderator-only lobby membership management; removal resolves the target's current lobby automatically because a player can only be in one active lobby per server. Adding a player also refreshes their OpenDota rank and win/loss data.
- `/lobby-seed <number> [count]` — developer-only test helper that adds up to 10 synthetic players with varied ranks for balance testing; requires `DEVELOPER_DISCORD_IDS`.
- `/lobby [number]` — list a numbered lobby (or your current lobby), including rank badges and Dota profile links for linked players.
- `/lobby-balance <number>` — list a numbered lobby split into balanced Radiant and Dire teams.
- `/register <friend_code>` — link an 8-digit Steam friend code and cache OpenDota rank and win/loss data.
- `/admin-role-tier create <name> <moderator>` — define a named tier for the current guild and choose whether it can manage the bot.
- `/admin-role-tier list` — list the current guild's configured tiers.
- `/admin-role-map <discord_role> <tier>` — map a Discord role to a configured tier in the current guild.
- `/warn`, `/ban`, `/unban`, `/history` — SQL-backed inhouse moderation history scoped to the current guild. Ban is an inhouse-only flag, not a Discord server ban.

## Workflow documentation

See [`docs/workflows.md`](docs/workflows.md) for Mermaid diagrams covering startup, command dispatch, guild setup, voice-channel player lookup, concurrent lobby management, moderation, and Steam data refresh.

## Multi-guild behavior

- Commands are registered globally, so every guild using the bot receives the same command surface. Discord may take time to propagate global command changes.
- For immediate development updates without removing the global commands, set `DISCORD_COMMAND_GUILD_ID` to a target guild ID. The same command set is registered in that guild as well and updates immediately.
- On startup, Housekeeper discovers every guild in the gateway `READY` payload and synchronizes each guild's roles and members.
- Guilds joined while the bot is running are synchronized through `guild_create`.
- Role tiers, role mappings, moderator authorization, infractions, and guild metadata are resolved using the current interaction's guild ID. A role mapped in one guild does not grant permissions in another.
- Discord guild administrators can configure their guild immediately. Users assigned a tier marked as a moderator can also manage configuration in that guild.
- `DEVELOPER_DISCORD_IDS` is a global support escape hatch for recovery and setup assistance; normal guild configuration does not depend on it.

## Setup

1. Create a PostgreSQL database and copy `.env.example` to `.env`.
2. Set `DISCORD_TOKEN`, `DATABASE_URL`, and optionally `DEVELOPER_DISCORD_IDS` or `DISCORD_COMMAND_GUILD_ID` for immediate development-guild command updates. `OPENDOTA_REQUESTS_PER_MINUTE` defaults to `60` to stay within the unauthenticated OpenDota limit; set it to the limit appropriate for your API key if applicable.
3. Invite the bot to each guild with the required slash-command, member, role, and voice-state permissions/intents.
4. Run `cargo run` (embedded SQLx migrations run at startup). On startup, the bot automatically provisions any missing Dota rank application emojis and discovers them; no emoji configuration or manual setup is required.
5. In each guild, an Administrator runs `/admin-role-tier create`, then `/admin-role-map` to connect Discord roles to the configured tiers.

## Development

```powershell
cargo fmt
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

The project intentionally keeps commands, repositories, models, and services in separate small modules. Commands are thin adapters; business rules live in services and SQL stays in repositories.
