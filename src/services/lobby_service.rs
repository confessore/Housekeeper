use crate::{
    database::{LobbyRepository, UserRepository},
    models::{Lobby, User},
    services::{
        rank_lookup_service::RankLookupService, role_resolver, whoishere_service::PlayerView,
    },
    utils::error::{AppError, AppResult},
};
use chrono::{Duration, Utc};
use sqlx::PgPool;

pub const CAPACITY: i64 = 10;
pub const DEFAULT_TTL_HOURS: i64 = 12;
pub const MAX_TTL_HOURS: i64 = 24;

pub(crate) async fn resolve_guild_id(pool: &PgPool, discord_guild_id: i64) -> AppResult<i64> {
    sqlx::query("INSERT INTO guilds(discord_guild_id,name) VALUES($1,$2) ON CONFLICT(discord_guild_id) DO NOTHING")
        .bind(discord_guild_id)
        .bind("Discord guild")
        .execute(pool)
        .await?;
    Ok(
        sqlx::query_scalar("SELECT id FROM guilds WHERE discord_guild_id=$1")
            .bind(discord_guild_id)
            .fetch_one(pool)
            .await?,
    )
}

pub fn ttl_hours(requested: Option<i64>) -> i64 {
    requested
        .unwrap_or(DEFAULT_TTL_HOURS)
        .clamp(1, MAX_TTL_HOURS)
}

pub fn validate_join(
    has_steam: bool,
    banned: bool,
    already_joined: bool,
    count: i64,
) -> AppResult<()> {
    if !has_steam {
        return Err(AppError::InvalidInput(
            "link your Steam account with /link-steam before joining the lobby".into(),
        ));
    }
    if banned {
        return Err(AppError::Forbidden);
    }
    if already_joined {
        return Err(AppError::InvalidInput(
            "you are already in this lobby".into(),
        ));
    }
    if count >= CAPACITY {
        return Err(AppError::InvalidInput(
            "the lobby is full (10 players)".into(),
        ));
    }
    Ok(())
}

pub async fn create(
    pool: &PgPool,
    guild_id: i64,
    creator: &str,
    requested_ttl: Option<i64>,
) -> AppResult<Lobby> {
    let guild_id = resolve_guild_id(pool, guild_id).await?;
    let expires_at = Utc::now() + Duration::hours(ttl_hours(requested_ttl));
    LobbyRepository::new(pool)
        .create(guild_id, creator, expires_at)
        .await?
        .ok_or_else(|| AppError::InvalidInput("unable to create a lobby".into()))
}

pub(crate) async fn resolve(pool: &PgPool, guild_id: i64, number: i32) -> AppResult<Lobby> {
    LobbyRepository::new(pool)
        .find_by_number(guild_id, number)
        .await?
        .ok_or_else(|| {
            AppError::InvalidInput(format!("lobby {number} was not found or has expired"))
        })
}

pub async fn join(
    pool: &PgPool,
    guild_id: i64,
    number: i32,
    user: &User,
    rank_service: &RankLookupService,
) -> AppResult<()> {
    let guild_id = resolve_guild_id(pool, guild_id).await?;
    let repo = LobbyRepository::new(pool);
    let lobby = resolve(pool, guild_id, number).await?;
    if let Some(current) = repo
        .find_current_for_member(guild_id, &user.discord_id)
        .await?
    {
        return Err(AppError::InvalidInput(format!(
            "you are already in lobby #{}",
            current.number
        )));
    }
    validate_join(
        user.friend_code.is_some(),
        user.inhouse_banned,
        false,
        repo.count(lobby.id).await?,
    )?;
    let snapshot = match rank_service.refresh(user).await {
        Ok(user) => user,
        Err(error) => {
            tracing::warn!(error = %error, discord_id = %user.discord_id, "could not refresh rank before lobby join; using cached data");
            user.clone()
        }
    };
    if !repo
        .add(
            lobby.id,
            &snapshot.discord_id,
            &snapshot.discord_id,
            snapshot.rank_tier,
            snapshot.wins,
            snapshot.losses,
        )
        .await?
    {
        return Err(AppError::InvalidInput(
            "the lobby filled up or you are already in it".into(),
        ));
    }
    Ok(())
}

pub async fn add(
    pool: &PgPool,
    guild_id: i64,
    number: i32,
    target: &User,
    actor: &str,
    rank_service: &RankLookupService,
) -> AppResult<()> {
    let guild_id = resolve_guild_id(pool, guild_id).await?;
    let repo = LobbyRepository::new(pool);
    let lobby = resolve(pool, guild_id, number).await?;
    if let Some(current) = repo
        .find_current_for_member(guild_id, &target.discord_id)
        .await?
    {
        return Err(AppError::InvalidInput(format!(
            "that player is already in lobby #{}",
            current.number
        )));
    }
    validate_join(
        target.friend_code.is_some(),
        target.inhouse_banned,
        false,
        repo.count(lobby.id).await?,
    )?;
    let snapshot = match rank_service.refresh(target).await {
        Ok(user) => user,
        Err(error) => {
            tracing::warn!(error = %error, discord_id = %target.discord_id, "could not refresh rank before lobby add; using cached data");
            target.clone()
        }
    };
    if !repo
        .add(
            lobby.id,
            &snapshot.discord_id,
            actor,
            snapshot.rank_tier,
            snapshot.wins,
            snapshot.losses,
        )
        .await?
    {
        return Err(AppError::InvalidInput(
            "the lobby filled up or that player is already in it".into(),
        ));
    }
    Ok(())
}

pub async fn leave(pool: &PgPool, guild_id: i64, discord_id: &str) -> AppResult<()> {
    let guild_id = resolve_guild_id(pool, guild_id).await?;
    let repo = LobbyRepository::new(pool);
    let lobby = repo
        .find_current_for_member(guild_id, discord_id)
        .await?
        .ok_or_else(|| AppError::InvalidInput("you are not in a lobby".into()))?;
    repo.remove(lobby.id, discord_id).await?;
    Ok(())
}

pub async fn remove(pool: &PgPool, guild_id: i64, number: i32, discord_id: &str) -> AppResult<()> {
    let guild_id = resolve_guild_id(pool, guild_id).await?;
    let lobby = resolve(pool, guild_id, number).await?;
    if !LobbyRepository::new(pool)
        .remove(lobby.id, discord_id)
        .await?
    {
        return Err(AppError::InvalidInput(
            "that player is not in the lobby".into(),
        ));
    }
    Ok(())
}

pub async fn close(pool: &PgPool, guild_id: i64, number: i32) -> AppResult<()> {
    let guild_id = resolve_guild_id(pool, guild_id).await?;
    if !LobbyRepository::new(pool).close(guild_id, number).await? {
        return Err(AppError::InvalidInput("that lobby is not open".into()));
    }
    Ok(())
}

pub async fn active(pool: &PgPool, guild_id: i64) -> AppResult<Vec<(Lobby, i64)>> {
    let guild_id = resolve_guild_id(pool, guild_id).await?;
    let repo = LobbyRepository::new(pool);
    repo.cleanup_expired(guild_id).await?;
    let mut result = Vec::new();
    for lobby in repo.list_active(guild_id).await? {
        result.push((lobby.clone(), repo.count(lobby.id).await?));
    }
    Ok(result)
}

pub async fn list(
    pool: &PgPool,
    guild_id: i64,
    number: Option<i32>,
    requester: &str,
) -> AppResult<(Lobby, Vec<PlayerView>)> {
    let discord_guild_id = guild_id;
    let guild_id = resolve_guild_id(pool, guild_id).await?;
    let repo = LobbyRepository::new(pool);
    let lobby = match number {
        Some(number) => resolve(pool, guild_id, number).await?,
        None => repo
            .find_current_for_member(guild_id, requester)
            .await?
            .ok_or_else(|| AppError::InvalidInput("specify a lobby number".into()))?,
    };
    let user_repo = UserRepository::new(pool.clone());
    let mut players = Vec::new();
    for member in repo.list(lobby.id).await? {
        if let Some(user) = user_repo.find_by_discord(&member.discord_id).await? {
            let role = role_resolver::resolve(pool, discord_guild_id, &member.discord_id).await?;
            players.push(crate::services::whoishere_service::view_with_stats(
                user,
                role,
                member.rank_tier,
                Some(member.wins),
                Some(member.losses),
            ));
        }
    }
    crate::services::whoishere_service::sort_players(&mut players);
    Ok((lobby, players))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unlinked_players() {
        assert!(validate_join(false, false, false, 0).is_err());
    }

    #[test]
    fn rejects_full_lobbies() {
        assert!(validate_join(true, false, false, CAPACITY).is_err());
    }

    #[test]
    fn rejects_duplicates_and_banned_players() {
        assert!(validate_join(true, false, true, 1).is_err());
        assert!(validate_join(true, true, false, 1).is_err());
    }

    #[test]
    fn clamps_lobby_lifetime() {
        assert_eq!(ttl_hours(None), DEFAULT_TTL_HOURS);
        assert_eq!(ttl_hours(Some(0)), 1);
        assert_eq!(ttl_hours(Some(100)), MAX_TTL_HOURS);
    }
}
