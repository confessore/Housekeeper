use crate::{
    discord::bot::DiscordBot,
    services::{lobby_seed_service, lobby_service, moderation::permission},
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};

pub fn register() -> CreateCommand {
    CreateCommand::new("lobby-seed")
        .description("Add synthetic players for balance testing (developer only)")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "number", "Lobby number")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "count",
                "Number of players (1-10)",
            )
            .required(false),
        )
}

pub async fn run(ctx: &Context, c: &CommandInteraction, bot: &DiscordBot) -> AppResult<()> {
    permission::require_developer(c, bot)?;
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
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
    let count = c
        .data
        .options
        .iter()
        .find(|option| option.name == "count")
        .and_then(|option| match option.value {
            CommandDataOptionValue::Integer(value) => Some(value),
            _ => None,
        })
        .unwrap_or(lobby_service::CAPACITY);
    let users = lobby_seed_service::seed(
        &bot.pool,
        guild.get() as i64,
        number,
        count,
        &bot.rank_service,
    )
    .await?;
    let roster = users
        .iter()
        .map(|user| {
            format!(
                "{} ({})",
                user.username,
                crate::models::Rank::from_tier(user.rank_tier)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(format!(
                    "Added {} synthetic players to lobby #{number}: {roster}",
                    users.len()
                ))
                .ephemeral(true),
        ),
    )
    .await
    .map_err(|error| AppError::Discord(error.to_string()))?;
    Ok(())
}
