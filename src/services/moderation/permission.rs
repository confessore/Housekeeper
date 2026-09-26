use crate::{
    discord::bot::DiscordBot,
    services::role_resolver,
    utils::error::{AppError, AppResult},
};
use serenity::all::{CommandInteraction, Permissions};
use sqlx::PgPool;

pub fn require_developer(interaction: &CommandInteraction, bot: &DiscordBot) -> AppResult<()> {
    if bot
        .settings
        .developer_discord_ids
        .iter()
        .any(|id| id == &interaction.user.id.to_string())
    {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

pub async fn require_moderator(pool: &PgPool, guild_id: i64, actor: &str) -> AppResult<()> {
    if role_resolver::resolve(pool, guild_id, actor)
        .await?
        .is_some_and(|tier| tier.is_moderator)
    {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

pub async fn require_role_manager(
    interaction: &CommandInteraction,
    bot: &DiscordBot,
    guild_id: i64,
) -> AppResult<()> {
    let is_developer = require_developer(interaction, bot).is_ok();
    let is_administrator = interaction.member.as_ref().is_some_and(|member| {
        member
            .permissions
            .is_some_and(|permissions| permissions.contains(Permissions::ADMINISTRATOR))
    });
    let is_moderator =
        role_resolver::resolve(&bot.pool, guild_id, &interaction.user.id.to_string())
            .await?
            .is_some_and(|tier| tier.is_moderator);
    if is_developer || is_administrator || is_moderator {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}
