use crate::{
    discord::bot::DiscordBot,
    services::role_resolver,
    utils::error::{AppError, AppResult},
};
use serenity::all::{CommandInteraction, Member, Permissions, User};
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
    require_role_manager_for(
        &bot.pool,
        bot,
        guild_id,
        &interaction.user,
        interaction.member.as_deref(),
    )
    .await
}

pub async fn require_role_manager_for(
    pool: &PgPool,
    bot: &DiscordBot,
    guild_id: i64,
    user: &User,
    member: Option<&Member>,
) -> AppResult<()> {
    let is_developer = bot
        .settings
        .developer_discord_ids
        .iter()
        .any(|id| id == &user.id.to_string());
    let is_administrator = member.is_some_and(|member| {
        member
            .permissions
            .is_some_and(|permissions| permissions.contains(Permissions::ADMINISTRATOR))
    });
    let is_moderator = role_resolver::resolve(pool, guild_id, &user.id.to_string())
        .await?
        .is_some_and(|tier| tier.is_moderator);
    if is_developer || is_administrator || is_moderator {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}
