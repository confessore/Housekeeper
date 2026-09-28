use crate::{
    discord::bot::DiscordBot,
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};
pub fn register() -> CreateCommand {
    CreateCommand::new("register")
        .description("Link your Steam friend code and refresh Dota data")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "friend_code",
                "Steam friend code",
            )
            .required(true),
        )
}
pub async fn run(ctx: &Context, c: &CommandInteraction, bot: &DiscordBot) -> AppResult<()> {
    let input = c
        .data
        .options
        .iter()
        .find(|o| o.name == "friend_code")
        .and_then(|o| o.value.as_str())
        .ok_or_else(|| AppError::InvalidInput("friend_code is required".into()))?;
    let display_name = crate::discord::member_name::interaction_name(&c.user, c.member.as_deref());
    crate::services::register_service::link(
        &bot.pool,
        &bot.rank_service,
        &c.user.id.to_string(),
        &display_name,
        input,
    )
    .await?;
    respond(
        ctx,
        c,
        "Steam account linked and Dota data refreshed.",
        true,
    )
    .await
}
async fn respond(
    ctx: &Context,
    c: &CommandInteraction,
    text: &str,
    ephemeral: bool,
) -> AppResult<()> {
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(text)
                .ephemeral(ephemeral),
        ),
    )
    .await
    .map_err(|e| AppError::Discord(e.to_string()))?;
    Ok(())
}
