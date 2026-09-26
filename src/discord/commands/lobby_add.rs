use super::moderation_common::require_role_manager;
use crate::{
    database::UserRepository,
    discord::bot::DiscordBot,
    services::lobby_service,
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};

pub fn register() -> CreateCommand {
    CreateCommand::new("lobby-add")
        .description("Add a player to a lobby")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "number", "Lobby number")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Player").required(true),
        )
}

pub async fn run(ctx: &Context, c: &CommandInteraction, bot: &DiscordBot) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    require_role_manager(c, bot, guild.get() as i64).await?;
    let number = c
        .data
        .options
        .iter()
        .find(|option| option.name == "number")
        .and_then(|option| match option.value {
            CommandDataOptionValue::Integer(value) => i32::try_from(value).ok(),
            _ => None,
        })
        .ok_or_else(|| AppError::InvalidInput("lobby number is required".into()))?;
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
    let user = UserRepository::new(bot.pool.clone())
        .find_by_discord(&target)
        .await?
        .ok_or_else(|| {
            AppError::InvalidInput("that player must link Steam before being added".into())
        })?;
    lobby_service::add(
        &bot.pool,
        guild.get() as i64,
        number,
        &user,
        &c.user.id.to_string(),
        &bot.rank_service,
    )
    .await?;
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content("Player added to the lobby."),
        ),
    )
    .await
    .map_err(|error| AppError::Discord(error.to_string()))?;
    Ok(())
}
