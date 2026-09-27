use crate::{
    discord::bot::DiscordBot,
    models::InfractionKind,
    services::moderation::{infractions, permission},
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, Context, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

pub async fn require_role_manager(
    interaction: &CommandInteraction,
    bot: &DiscordBot,
    discord_guild_id: i64,
) -> AppResult<()> {
    permission::require_role_manager(interaction, bot, discord_guild_id).await
}

pub async fn run_infraction(
    ctx: &Context,
    c: &CommandInteraction,
    b: &DiscordBot,
    kind: InfractionKind,
) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    let target = c
        .data
        .options
        .iter()
        .find(|o| o.name == "user")
        .and_then(|o| match o.value {
            CommandDataOptionValue::User(id) => Some(id.to_string()),
            _ => None,
        })
        .ok_or_else(|| AppError::InvalidInput("user required".into()))?;
    let reason = c
        .data
        .options
        .iter()
        .find(|o| o.name == "reason")
        .and_then(|o| o.value.as_str())
        .unwrap_or("No reason provided");
    infractions::record(
        &b.pool,
        guild.get() as i64,
        &c.user.id.to_string(),
        &target,
        kind,
        reason,
    )
    .await?;
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("Infraction recorded.")
                .ephemeral(true),
        ),
    )
    .await
    .map_err(|e| AppError::Discord(e.to_string()))?;
    Ok(())
}
