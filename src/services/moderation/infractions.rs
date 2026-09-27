use crate::{
    database::{InfractionRepository, UserRepository},
    models::{Infraction, InfractionKind},
    utils::error::AppResult,
};
use sqlx::PgPool;
pub async fn record(
    pool: &PgPool,
    discord_guild_id: i64,
    actor: &str,
    target: &str,
    kind: InfractionKind,
    reason: &str,
) -> AppResult<()> {
    crate::services::moderation::permission::require_moderator(pool, discord_guild_id, actor)
        .await?;
    InfractionRepository::new(pool.clone())
        .add(target, discord_guild_id, kind, reason, actor)
        .await?;
    if matches!(kind, InfractionKind::Ban | InfractionKind::Unban) {
        UserRepository::new(pool.clone())
            .set_banned(target, matches!(kind, InfractionKind::Ban))
            .await?;
    }
    Ok(())
}
pub async fn history(
    pool: &PgPool,
    discord_guild_id: i64,
    actor: &str,
    target: &str,
) -> AppResult<Vec<Infraction>> {
    crate::services::moderation::permission::require_moderator(pool, discord_guild_id, actor)
        .await?;
    InfractionRepository::new(pool.clone())
        .history(target, discord_guild_id)
        .await
}
