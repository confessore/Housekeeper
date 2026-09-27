use crate::{
    models::{Lobby, LobbyMember},
    utils::error::{AppError, AppResult},
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

pub struct LobbyRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> LobbyRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn cleanup_expired(&self, guild_id: i64) -> AppResult<()> {
        sqlx::query("DELETE FROM lobbies WHERE guild_id=$1 AND expires_at <= NOW()")
            .bind(guild_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    pub async fn create(
        &self,
        guild_id: i64,
        created_by: &str,
        expires_at: DateTime<Utc>,
    ) -> AppResult<Option<Lobby>> {
        let mut tx = self.pool.begin().await?;
        let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
            .bind(guild_id)
            .fetch_one(&mut *tx)
            .await?;
        if !acquired {
            return Err(AppError::InvalidInput(
                "another lobby is currently being created for this server; try again shortly"
                    .into(),
            ));
        }
        sqlx::query("DELETE FROM lobbies WHERE guild_id=$1 AND expires_at <= NOW()")
            .bind(guild_id)
            .execute(&mut *tx)
            .await?;
        let number: i32 = sqlx::query_scalar(
            "SELECT candidate::integer
             FROM generate_series(
                 1::bigint,
                 LEAST(
                     COALESCE((SELECT MAX(number)::bigint FROM lobbies WHERE guild_id=$1), 0) + 1,
                     2147483647
                 )
             ) AS candidate
             WHERE NOT EXISTS (SELECT 1 FROM lobbies WHERE guild_id=$1 AND lobbies.number=candidate)
             ORDER BY candidate
             LIMIT 1",
        )
        .bind(guild_id)
        .fetch_one(&mut *tx)
        .await?;
        let row = sqlx::query_as::<_, (i64, i64, i32, DateTime<Utc>, DateTime<Utc>)>(
            "INSERT INTO lobbies(guild_id,number,created_by_discord_id,expires_at)
             VALUES ($1,$2,$3,$4)
             RETURNING id,guild_id,number,created_at,expires_at",
        )
        .bind(guild_id)
        .bind(number)
        .bind(created_by)
        .bind(expires_at)
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(row.map(lobby_from_row))
    }

    pub async fn find_by_number(&self, guild_id: i64, number: i32) -> AppResult<Option<Lobby>> {
        let row = sqlx::query_as::<_, (i64, i64, i32, DateTime<Utc>, DateTime<Utc>)>(
            "SELECT id,guild_id,number,created_at,expires_at
             FROM lobbies WHERE guild_id=$1 AND number=$2 AND expires_at > NOW()",
        )
        .bind(guild_id)
        .bind(number)
        .fetch_optional(self.pool)
        .await?;
        Ok(row.map(lobby_from_row))
    }

    pub async fn find_current_for_member(
        &self,
        guild_id: i64,
        discord_id: &str,
    ) -> AppResult<Option<Lobby>> {
        let row = sqlx::query_as::<_, (i64, i64, i32, DateTime<Utc>, DateTime<Utc>)>(
            "SELECT l.id,l.guild_id,l.number,l.created_at,l.expires_at
             FROM lobbies l JOIN lobby_members m ON m.lobby_id=l.id
             WHERE l.guild_id=$1 AND m.discord_id=$2 AND l.expires_at > NOW()",
        )
        .bind(guild_id)
        .bind(discord_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(row.map(lobby_from_row))
    }

    pub async fn list_active(&self, guild_id: i64) -> AppResult<Vec<Lobby>> {
        let rows = sqlx::query_as::<_, (i64, i64, i32, DateTime<Utc>, DateTime<Utc>)>(
            "SELECT id,guild_id,number,created_at,expires_at
             FROM lobbies WHERE guild_id=$1 AND expires_at > NOW()
             ORDER BY number",
        )
        .bind(guild_id)
        .fetch_all(self.pool)
        .await?;
        Ok(rows.into_iter().map(lobby_from_row).collect())
    }

    pub async fn close(&self, guild_id: i64, number: i32) -> AppResult<bool> {
        let result = sqlx::query(
            "UPDATE lobbies SET expires_at=NOW()
             WHERE guild_id=$1 AND number=$2 AND expires_at > NOW()",
        )
        .bind(guild_id)
        .bind(number)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn add(
        &self,
        lobby_id: i64,
        discord_id: &str,
        added_by: &str,
        rank_tier: Option<i32>,
        wins: i32,
        losses: i32,
    ) -> AppResult<bool> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(-lobby_id)
            .execute(&mut *tx)
            .await?;
        let result = sqlx::query(
            "INSERT INTO lobby_members(lobby_id,discord_id,added_by_discord_id,rank_tier,wins,losses)
             SELECT $1,$2,$3,$4,$5,$6
             WHERE EXISTS (SELECT 1 FROM lobbies WHERE id=$1 AND expires_at > NOW())
               AND (SELECT COUNT(*) FROM lobby_members WHERE lobby_id=$1) < 10
             ON CONFLICT (lobby_id,discord_id) DO NOTHING",
        )
        .bind(lobby_id)
        .bind(discord_id)
        .bind(added_by)
        .bind(rank_tier)
        .bind(wins)
        .bind(losses)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn remove(&self, lobby_id: i64, discord_id: &str) -> AppResult<bool> {
        let result = sqlx::query("DELETE FROM lobby_members WHERE lobby_id=$1 AND discord_id=$2")
            .bind(lobby_id)
            .bind(discord_id)
            .execute(self.pool)
            .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn count(&self, lobby_id: i64) -> AppResult<i64> {
        Ok(
            sqlx::query_scalar("SELECT COUNT(*) FROM lobby_members WHERE lobby_id=$1")
                .bind(lobby_id)
                .fetch_one(self.pool)
                .await?,
        )
    }

    pub async fn list(&self, lobby_id: i64) -> AppResult<Vec<LobbyMember>> {
        Ok(sqlx::query_as::<_, (String, Option<i32>, i32, i32)>(
            "SELECT discord_id,rank_tier,wins,losses FROM lobby_members
             WHERE lobby_id=$1 ORDER BY joined_at,discord_id",
        )
        .bind(lobby_id)
        .fetch_all(self.pool)
        .await?
        .into_iter()
        .map(|row| LobbyMember {
            discord_id: row.0,
            rank_tier: row.1,
            wins: row.2,
            losses: row.3,
        })
        .collect())
    }
}

fn lobby_from_row(row: (i64, i64, i32, DateTime<Utc>, DateTime<Utc>)) -> Lobby {
    Lobby {
        id: row.0,
        number: row.2,
        expires_at: row.4,
    }
}
