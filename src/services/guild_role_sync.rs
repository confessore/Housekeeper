use crate::{
    database::RoleRepository,
    utils::error::{AppError, AppResult},
};
use serenity::all::{Context, GuildId};
use sqlx::PgPool;
pub async fn sync(pool: &PgPool, ctx: &Context, guild_id: GuildId) -> AppResult<()> {
    let guild = guild_id
        .to_partial_guild(&ctx.http)
        .await
        .map_err(|e| AppError::Discord(e.to_string()))?;
    sqlx::query("INSERT INTO guilds(discord_guild_id,name) VALUES($1,$2) ON CONFLICT(discord_guild_id) DO UPDATE SET name=EXCLUDED.name")
        .bind(guild_id.get() as i64)
        .bind(&guild.name)
        .execute(pool)
        .await?;
    let roles = guild_id
        .roles(&ctx.http)
        .await
        .map_err(|e| AppError::Discord(e.to_string()))?;
    let repo = RoleRepository::new(pool.clone());
    for (id, role) in roles {
        repo.cache_role(
            guild_id.get() as i64,
            &id.to_string(),
            &role.name,
            role.position as i64,
        )
        .await?;
    }
    sync_members(pool, ctx, guild_id).await
}
pub async fn sync_members(pool: &PgPool, ctx: &Context, guild_id: GuildId) -> AppResult<()> {
    let members = guild_id
        .members(&ctx.http, None, None)
        .await
        .map_err(|e| AppError::Discord(e.to_string()))?;
    for member in members {
        let roles = member
            .roles
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        sync_member(pool, &member.user.id.to_string(), &roles).await?;
    }
    Ok(())
}
pub async fn sync_member(pool: &PgPool, user_id: &str, role_ids: &[String]) -> AppResult<()> {
    RoleRepository::new(pool.clone())
        .replace_member_roles(user_id, role_ids)
        .await
}
