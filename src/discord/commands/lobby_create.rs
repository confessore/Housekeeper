use crate::{
    discord::bot::DiscordBot,
    services::lobby_service,
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, EditInteractionResponse,
};

pub fn register() -> CreateCommand {
    CreateCommand::new("lobby-create")
        .description("Create an inhouse lobby")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "hours",
                "Lifetime in hours (1-24)",
            )
            .required(false),
        )
}

pub async fn run(ctx: &Context, c: &CommandInteraction, bot: &DiscordBot) -> AppResult<()> {
    c.defer(ctx)
        .await
        .map_err(|error| AppError::Discord(error.to_string()))?;
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    let hours = c
        .data
        .options
        .iter()
        .find(|option| option.name == "hours")
        .and_then(|option| match option.value {
            CommandDataOptionValue::Integer(value) => Some(value),
            _ => None,
        });
    let lobby =
        lobby_service::create(&bot.pool, guild.get() as i64, &c.user.id.to_string(), hours).await?;
    c.edit_response(
        ctx,
        EditInteractionResponse::new().content(format!(
            "Created lobby #{} for up to {} hours.",
            lobby.number,
            lobby_service::ttl_hours(hours)
        )),
    )
    .await
    .map_err(|error| AppError::Discord(error.to_string()))?;
    Ok(())
}
