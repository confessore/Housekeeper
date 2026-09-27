use super::moderation_common::require_role_manager;
use crate::{
    discord::bot::DiscordBot,
    services::lobby_service,
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption,
};

pub fn register() -> CreateCommand {
    CreateCommand::new("lobby-remove")
        .description("Remove a player from their current lobby")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Player").required(true),
        )
}

pub async fn run(ctx: &Context, c: &CommandInteraction, bot: &DiscordBot) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    require_role_manager(c, bot, guild.get() as i64).await?;
    let target = c
        .data
        .options
        .iter()
        .find(|option| option.name == "user")
        .and_then(|option| match option.value {
            CommandDataOptionValue::User(id) => Some(id.to_string()),
            _ => None,
        })
        .ok_or_else(|| AppError::InvalidInput("user is required".into()))?;
    let lobby = lobby_service::remove_from_current(&bot.pool, guild.get() as i64, &target).await?;
    crate::discord::components::send_lobby_panel(ctx, c, bot, lobby.number, lobby.id, false).await
}
