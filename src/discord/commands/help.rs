use crate::{
    discord::bot::DiscordBot,
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};

const HELP_TEXT: &str = "**Housekeeper commands**

Use `/help command:<command>` for detailed usage, options, and permissions for one command.

`/whoishere [balance]` — List non-bot players in your current voice channel with rank, guild tier, and record.
`/link-steam <friend_code>` — Link your 8-digit Steam friend code and refresh Dota data.
`/lobby-create [hours]`, `/lobby-list`, `/lobby-close <number>` — Manage numbered lobbies. Numbers are assigned automatically and reused when available; lobbies expire within 24 hours.
`/lobby-join <number>`, `/lobby-leave` — Join or leave a numbered lobby. Each player can be in one lobby per server.
`/lobby-add <number> <user>`, `/lobby-remove <number> <user>` — Moderator-only lobby membership management.
`/lobby-seed <number> [count]` — Developer-only synthetic roster for balance testing.
`/lobby [number]` — Show a numbered lobby, or your current lobby.
`/lobby-balance <number>` — Show a numbered lobby split into balanced Radiant and Dire teams.
`/admin-role-tier`, `/admin-role-map` — Configure guild role tiers and role mappings.
`/warn`, `/ban`, `/unban`, `/history` — Manage and inspect inhouse moderation history.

Guild Administrators or members in a moderator tier can configure the guild. Moderation and lobby management commands require the configured moderator permission.";

const COMMAND_HELP: &[(&str, &str)] = &[
    (
        "whoishere",
        "**/whoishere**\nList non-bot players in your current voice channel.\n\n**Options**\n• `balance` (optional) — Split the players into two balanced teams.\n\nThis is a voice-channel scan, not a lobby roster.",
    ),
    (
        "link-steam",
        "**/link-steam <friend_code>**\nLink your 8-digit Steam friend code and refresh your cached rank and win/loss data. The code may be entered as `12345678` or `1234-5678`.",
    ),
    (
        "lobby-create",
        "**/lobby-create [hours]**\nCreate a lobby in this server. Housekeeper assigns the smallest available number and reuses numbers after lobbies close or expire.\n\n**Options**\n• `hours` (optional) — Lifetime from 1 to 24 hours; defaults to 12.\n\nEach lobby holds up to 10 players.",
    ),
    (
        "lobby-list",
        "**/lobby-list**\nList every active numbered lobby in this server, including its player count and remaining lifetime.",
    ),
    (
        "lobby-close",
        "**/lobby-close <number>**\nClose a numbered lobby immediately.\n\n**Permissions**\nRequires the configured moderator permission. Closing a lobby removes its active membership from consideration.",
    ),
    (
        "lobby-join",
        "**/lobby-join <number>**\nJoin an active numbered lobby.\n\n**Requirements**\n• Steam must be linked.\n• You must not be inhouse-banned.\n• You may only belong to one active lobby in this server.\n• The lobby must have fewer than 10 players.",
    ),
    (
        "lobby-leave",
        "**/lobby-leave**\nLeave your current active lobby. You do not need to provide its name.",
    ),
    (
        "lobby-add",
        "**/lobby-add <number> <user>**\nAdd a Steam-linked player to a numbered lobby.\n\n**Permissions**\nRequires the configured moderator permission. A player may only belong to one active lobby in this server.",
    ),
    (
        "lobby-remove",
        "**/lobby-remove <number> <user>**\nRemove a player from a numbered lobby.\n\n**Permissions**\nRequires the configured moderator permission.",
    ),
    (
        "lobby",
        "**/lobby [number]**\nShow a lobby roster. If `number` is omitted, your current lobby is shown.\n\n**Options**\n• `number` (optional) — Numbered lobby to display.",
    ),
    (
        "lobby-balance",
        "**/lobby-balance <number>**\nShow a numbered lobby split into balanced Radiant and Dire teams.\n\n**Options**\n• `number` (required) — Numbered lobby to balance.",
    ),
    (
        "lobby-seed",
        "**/lobby-seed <number> [count]**\nAdd synthetic players with varied rank tiers to an existing lobby for balance testing.\n\n**Options**\n• `number` (required) — Existing lobby number to seed.\n• `count` (optional) — Number of synthetic players from 1 to 10; defaults to 10.\n\n**Permissions**\nDeveloper-only. The caller's Discord ID must be listed in `DEVELOPER_DISCORD_IDS`.",
    ),
    (
        "admin-role-tier",
        "**/admin-role-tier**\nCreate or list this server's role tiers.\n\n**Permissions**\nRequires Administrator or the configured role-manager permission. The `create` subcommand defines whether a tier can manage Housekeeper.",
    ),
    (
        "admin-role-map",
        "**/admin-role-map <discord_role> <tier>**\nMap a Discord role to a configured role tier in this server.\n\n**Permissions**\nRequires Administrator or the configured role-manager permission.",
    ),
    (
        "warn",
        "**/warn <user> <reason>**\nRecord an inhouse warning for a player in this server.\n\n**Permissions**\nRequires the configured moderator permission.",
    ),
    (
        "ban",
        "**/ban <user> <reason>**\nMark a player as inhouse-banned. This does not ban them from the Discord server.\n\n**Permissions**\nRequires the configured moderator permission.",
    ),
    (
        "unban",
        "**/unban <user> <reason>**\nRemove a player's inhouse ban.\n\n**Permissions**\nRequires the configured moderator permission.",
    ),
    (
        "history",
        "**/history <user>**\nShow a player's inhouse moderation history for this server.\n\n**Permissions**\nRequires the configured moderator permission.",
    ),
];

pub fn register() -> CreateCommand {
    let mut option = CreateCommandOption::new(
        CommandOptionType::String,
        "command",
        "Command to explain; omit for the overview",
    )
    .required(false);
    for (name, _) in COMMAND_HELP {
        option = option.add_string_choice(*name, *name);
    }
    CreateCommand::new("help")
        .description("Explain a Housekeeper command")
        .add_option(option)
}

pub async fn run(ctx: &Context, command: &CommandInteraction, _bot: &DiscordBot) -> AppResult<()> {
    let selected = command
        .data
        .options
        .iter()
        .find(|option| option.name == "command")
        .and_then(|option| match &option.value {
            CommandDataOptionValue::String(value) => Some(value.as_str()),
            _ => None,
        });
    let content = selected.and_then(command_help).unwrap_or(HELP_TEXT);
    command
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(content),
            ),
        )
        .await
        .map_err(|error| AppError::Discord(error.to_string()))?;
    Ok(())
}

fn command_help(name: &str) -> Option<&'static str> {
    COMMAND_HELP
        .iter()
        .find(|(command, _)| *command == name)
        .map(|(_, help)| *help)
}

#[cfg(test)]
mod tests {
    use super::{command_help, COMMAND_HELP, HELP_TEXT};

    #[test]
    fn help_text_fits_discord_message_limit() {
        assert!(HELP_TEXT.chars().count() <= 2_000);
        assert!(COMMAND_HELP
            .iter()
            .all(|(_, help)| help.chars().count() <= 2_000));
    }

    #[test]
    fn every_registered_command_has_details() {
        for (command, _) in COMMAND_HELP {
            assert!(
                command_help(command).is_some(),
                "missing help for {command}"
            );
        }
    }
}
