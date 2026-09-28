use crate::{
    discord::bot::DiscordBot,
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};

const HELP_TEXT: &str = "**Housekeeper commands**

Use `/housekeeper` for a private navigation hub, or `/help command:<command>` for detailed usage, options, permissions, and available buttons.

`/housekeeper` — Open private navigation for voice lookup, lobby list/current/create, Steam linking, and help. Shared lobby and voice panels are reused in the channel when possible.
`/whoishere [balance]` — List non-bot players in your current voice channel with rank, guild tier, and record. The response includes Balance teams and Refresh buttons.
`/register <friend_code>` — Link your Steam friend code and refresh Dota data.
`/lobby-create [hours]`, `/lobby-list`, `/lobby-close <number>` — Manage numbered lobbies. Numbers are assigned automatically and reused when available; lobbies expire within 24 hours.
`/lobby-join <number>`, `/lobby-leave` — Join or leave a numbered lobby. Each player can be in one lobby per server.
`/lobby-add <number> <user>`, `/lobby-remove <user>` — Moderator-only lobby membership management. A player can only be in one active lobby per server, so removal resolves the target's current lobby automatically.
`/lobby-seed <number> [count]` — Developer-only synthetic roster for balance testing.
`/lobby [number]` — Show a numbered lobby, or your current lobby.
`/lobby-balance <number>` — Show a numbered lobby split into balanced Radiant and Dire teams.
`/admin-role-tier`, `/admin-role-map` — Create, list, map, and remove guild role tiers and role mappings.
`/warn`, `/ban`, `/unban`, `/history` — Manage and inspect inhouse moderation history.

Guild Administrators or members in a moderator tier can configure the guild. Moderation and lobby management commands require the configured moderator permission.";

const COMMAND_HELP: &[(&str, &str)] = &[
    (
        "housekeeper",
        "**/housekeeper**\nOpen a private button hub.\n\n**Navigation**\n• `Who is here` — Show your current voice channel roster.\n• `Lobbies` — Open your private active-lobby list.\n• `My lobby` — Open your current lobby.\n• `Create lobby` — Create and publish a shared lobby panel.\n• `Link Steam` — Open the Steam registration form.\n\nThe hub is private to you. Shared lobby and voice panels are reused in the channel when possible.\n\nSlash commands remain available for every workflow.",
    ),
    (
        "whoishere",
        "**/whoishere**\nList non-bot players in your current voice channel. The response is shared in the channel for your user and includes Balance teams and Refresh buttons.\n\n**Options**\n• `balance` (optional) — Split the players into two balanced teams. You can also use the `Balance teams` button.\n\nThis is a voice-channel scan, not a lobby roster.",
    ),
    (
        "register",
        "**/register <friend_code>**\nLink your Steam friend code and refresh your cached rank and win/loss data. The code may be entered as digits, with an optional dash.",
    ),
    (
        "lobby-create",
        "**/lobby-create [hours]**\nCreate a lobby in this server. Housekeeper assigns the smallest available number and reuses numbers after lobbies close or expire. The response includes shared Join, Leave, Balance, Refresh, and Manage buttons.\n\n**Options**\n• `hours` (optional) — Lifetime from 1 to 24 hours; defaults to 12.\n\nEach lobby holds up to 10 players.",
    ),
    (
        "lobby-list",
        "**/lobby-list**\nList every active numbered lobby in this server, including its player count and remaining lifetime. Only you can see the list. Use `Open #N` to open the shared lobby panel for that lobby or `Create lobby` to start one.",
    ),
    (
        "lobby-close",
        "**/lobby-close <number>**\nClose a numbered lobby immediately.\n\n**Permissions**\nRequires the configured moderator permission. Closing a lobby removes its active membership from consideration.",
    ),
    (
        "lobby-join",
        "**/lobby-join <number>**\nJoin an active numbered lobby. The response opens the same shared lobby panel as the `Join` button.\n\n**Requirements**\n• Steam must be linked.\n• You must not be inhouse-banned.\n• You may only belong to one active lobby in this server.\n• The lobby must have fewer than 10 players.",
    ),
    (
        "lobby-leave",
        "**/lobby-leave**\nLeave your current active lobby. You do not need to provide its name. The response opens the same shared lobby panel as the `Leave` button.",
    ),
    (
        "lobby-add",
        "**/lobby-add <number> <user>**\nAdd a Steam-linked player to a numbered lobby. The response opens the same shared lobby panel as the `Manage` controls.\n\n**Permissions**\nRequires the configured moderator permission. A player may only belong to one active lobby in this server.",
    ),
    (
        "lobby-remove",
        "**/lobby-remove <user>**\nRemove a player from their current active lobby. The response opens the same shared lobby panel as the `Manage` controls; the lobby number is resolved automatically because a player can only belong to one active lobby per server.\n\n**Permissions**\nRequires the configured moderator permission.",
    ),
    (
        "lobby",
        "**/lobby [number]**\nShow a lobby roster. If `number` is omitted, your current lobby is shown. The response includes shared `Join`, `Leave`, `Balance teams`, `Refresh`, and `Manage` buttons.\n\n**Options**\n• `number` (optional) — Numbered lobby to display.",
    ),
    (
        "lobby-balance",
        "**/lobby-balance <number>**\nShow a numbered lobby split into balanced Radiant and Dire teams. The response includes the same shared lobby controls and can toggle the team display.\n\n**Options**\n• `number` (required) — Numbered lobby to balance.",
    ),
    (
        "lobby-seed",
        "**/lobby-seed <number> [count]**\nAdd synthetic players with varied rank tiers to an existing lobby for balance testing.\n\n**Options**\n• `number` (required) — Existing lobby number to seed.\n• `count` (optional) — Number of synthetic players from 1 to 10; defaults to 10.\n\n**Permissions**\nDeveloper-only. The caller's Discord ID must be listed in `DEVELOPER_DISCORD_IDS`.",
    ),
    (
        "admin-role-tier",
        "**/admin-role-tier**\nCreate, list, or remove this server's role tiers.\n\n**Subcommands**\n• `create` — Add a tier and define whether it can manage Housekeeper.\n• `list` — Show configured tiers.\n• `remove` — Remove a tier by name. If it has role mappings, re-run with `confirm:true` to cascade-delete them.\n\n**Permissions**\nRequires Administrator or the configured role-manager permission.",
    ),
    (
        "admin-role-map",
        "**/admin-role-map**\nManage Discord role mappings for configured tiers.\n\n**Subcommands**\n• `set <discord_role> <tier>` — Create or update a mapping.\n• `remove <discord_role>` — Remove a mapping.\n\n**Permissions**\nRequires Administrator or the configured role-manager permission.",
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
