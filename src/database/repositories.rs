use crate::models::{Infraction, InfractionKind, RoleTier, User};
use crate::utils::error::AppResult;
use sqlx::PgPool;

#[derive(Clone)]
pub struct UserRepository {
    pub pool: PgPool,
}
impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    pub async fn find_or_create(&self, discord_id: &str, username: &str) -> AppResult<User> {
        let row=sqlx::query_as::<_,(i64,String,String,Option<String>,Option<i32>,i32,i32,bool)>("INSERT INTO users (discord_id,username) VALUES ($1,$2) ON CONFLICT (discord_id) DO UPDATE SET username=EXCLUDED.username,updated_at=NOW() RETURNING id,discord_id,username,friend_code,rank_tier,wins,losses,inhouse_banned").bind(discord_id).bind(username).fetch_one(&self.pool).await?;
        Ok(User {
            id: row.0,
            discord_id: row.1,
            username: row.2,
            friend_code: row.3,
            rank_tier: row.4,
            wins: row.5,
            losses: row.6,
            inhouse_banned: row.7,
        })
    }
    pub async fn upsert_synthetic(
        &self,
        discord_id: &str,
        username: &str,
        rank_tier: Option<i32>,
        wins: i32,
        losses: i32,
    ) -> AppResult<User> {
        let row = sqlx::query_as::<
            _,
            (
                i64,
                String,
                String,
                Option<String>,
                Option<i32>,
                i32,
                i32,
                bool,
            ),
        >(
            "INSERT INTO users (discord_id,username,friend_code,rank_tier,wins,losses)
             VALUES ($1,$2,$1,$3,$4,$5)
             ON CONFLICT (discord_id) DO UPDATE SET
                 username=EXCLUDED.username, friend_code=EXCLUDED.friend_code,
                 rank_tier=EXCLUDED.rank_tier, wins=EXCLUDED.wins,
                 losses=EXCLUDED.losses, inhouse_banned=false, updated_at=NOW()
             RETURNING id,discord_id,username,friend_code,rank_tier,wins,losses,inhouse_banned",
        )
        .bind(discord_id)
        .bind(username)
        .bind(rank_tier)
        .bind(wins)
        .bind(losses)
        .fetch_one(&self.pool)
        .await?;
        Ok(User {
            id: row.0,
            discord_id: row.1,
            username: row.2,
            friend_code: row.3,
            rank_tier: row.4,
            wins: row.5,
            losses: row.6,
            inhouse_banned: row.7,
        })
    }

    pub async fn find_by_discord(&self, discord_id: &str) -> AppResult<Option<User>> {
        let row = sqlx::query_as::<_, (i64, String, String, Option<String>, Option<i32>, i32, i32, bool)>(
            "SELECT id,discord_id,username,friend_code,rank_tier,wins,losses,inhouse_banned FROM users WHERE discord_id=$1",
        )
        .bind(discord_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| User {
            id: row.0,
            discord_id: row.1,
            username: row.2,
            friend_code: row.3,
            rank_tier: row.4,
            wins: row.5,
            losses: row.6,
            inhouse_banned: row.7,
        }))
    }

    pub async fn update_friend_code(
        &self,
        id: i64,
        friend_code: &str,
        rank: Option<i32>,
        wins: i32,
        losses: i32,
    ) -> AppResult<()> {
        sqlx::query("UPDATE users SET friend_code=$1,rank_tier=$2,wins=$3,losses=$4,rank_synced_at=NOW(),updated_at=NOW() WHERE id=$5").bind(friend_code).bind(rank).bind(wins).bind(losses).bind(id).execute(&self.pool).await?;
        Ok(())
    }
    pub async fn set_banned(&self, discord_id: &str, banned: bool) -> AppResult<()> {
        sqlx::query("UPDATE users SET inhouse_banned=$1,updated_at=NOW() WHERE discord_id=$2")
            .bind(banned)
            .bind(discord_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct RoleRepository {
    pub pool: PgPool,
}
impl RoleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    pub async fn create_tier(
        &self,
        guild_id: i64,
        name: &str,
        is_moderator: bool,
    ) -> AppResult<RoleTier> {
        let row = sqlx::query_as::<_, (i64, i64, String, bool)>(
            "INSERT INTO role_tiers(guild_id,name,is_moderator) VALUES($1,$2,$3) RETURNING id,guild_id,name,is_moderator",
        )
        .bind(guild_id)
        .bind(name)
        .bind(is_moderator)
        .fetch_one(&self.pool)
        .await?;
        Ok(RoleTier {
            id: row.0,
            guild_id: row.1,
            name: row.2,
            is_moderator: row.3,
        })
    }
    pub async fn list_tiers(&self, guild_id: i64) -> AppResult<Vec<RoleTier>> {
        let rows = sqlx::query_as::<_, (i64, i64, String, bool)>(
            "SELECT id,guild_id,name,is_moderator FROM role_tiers WHERE guild_id=$1 ORDER BY name",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| RoleTier {
                id: r.0,
                guild_id: r.1,
                name: r.2,
                is_moderator: r.3,
            })
            .collect())
    }
    pub async fn find_tier_by_name(
        &self,
        guild_id: i64,
        name: &str,
    ) -> AppResult<Option<RoleTier>> {
        let row = sqlx::query_as::<_, (i64, i64, String, bool)>(
            "SELECT id,guild_id,name,is_moderator FROM role_tiers WHERE guild_id=$1 AND LOWER(name)=LOWER($2)",
        )
        .bind(guild_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| RoleTier {
            id: r.0,
            guild_id: r.1,
            name: r.2,
            is_moderator: r.3,
        }))
    }
    pub async fn map_role(&self, guild_id: i64, role_id: &str, tier_id: i64) -> AppResult<()> {
        sqlx::query("INSERT INTO role_mappings(guild_id,discord_role_id,role_tier_id) VALUES($1,$2,$3) ON CONFLICT(guild_id,discord_role_id) DO UPDATE SET role_tier_id=EXCLUDED.role_tier_id")
            .bind(guild_id)
            .bind(role_id)
            .bind(tier_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn resolve(&self, guild_id: i64, user_id: &str) -> AppResult<Option<RoleTier>> {
        let row=sqlx::query_as::<_,(i64,i64,String,bool)>("SELECT rt.id,rt.guild_id,rt.name,rt.is_moderator FROM member_roles_cache mc JOIN role_mappings rm ON rm.discord_role_id=mc.discord_role_id JOIN role_tiers rt ON rt.id=rm.role_tier_id JOIN guilds g ON g.id=rm.guild_id AND g.discord_guild_id=$1 JOIN discord_roles_cache rc ON rc.discord_role_id=mc.discord_role_id WHERE mc.user_discord_id=$2 ORDER BY rc.position DESC LIMIT 1").bind(guild_id).bind(user_id).fetch_optional(&self.pool).await?;
        Ok(row.map(|r| RoleTier {
            id: r.0,
            guild_id: r.1,
            name: r.2,
            is_moderator: r.3,
        }))
    }
    pub async fn replace_member_roles(&self, user_id: &str, role_ids: &[String]) -> AppResult<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM member_roles_cache WHERE user_discord_id=$1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        for role in role_ids {
            sqlx::query("INSERT INTO member_roles_cache(user_discord_id,discord_role_id) VALUES($1,$2) ON CONFLICT DO NOTHING").bind(user_id).bind(role).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn cache_role(
        &self,
        guild_id: i64,
        role_id: &str,
        name: &str,
        position: i64,
    ) -> AppResult<()> {
        sqlx::query("INSERT INTO discord_roles_cache(discord_role_id,guild_id,name,position) VALUES($1,$2,$3,$4) ON CONFLICT(discord_role_id) DO UPDATE SET name=EXCLUDED.name,position=EXCLUDED.position,updated_at=NOW()").bind(role_id).bind(guild_id).bind(name).bind(position).execute(&self.pool).await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct InfractionRepository {
    pub pool: PgPool,
}
impl InfractionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    pub async fn add(
        &self,
        target: &str,
        guild_id: i64,
        kind: InfractionKind,
        reason: &str,
        actor: &str,
    ) -> AppResult<()> {
        sqlx::query("INSERT INTO infractions(target_discord_id,guild_id,kind,reason,issued_by_discord_id) VALUES($1,$2,$3,$4,$5)").bind(target).bind(guild_id).bind(kind.as_str()).bind(reason).bind(actor).execute(&self.pool).await?;
        Ok(())
    }
    pub async fn history(&self, target: &str, guild_id: i64) -> AppResult<Vec<Infraction>> {
        let rows=sqlx::query_as::<_,(i64,String,String,String,String,chrono::DateTime<chrono::Utc>)>("SELECT id,target_discord_id,kind,reason,issued_by_discord_id,created_at FROM infractions WHERE target_discord_id=$1 AND guild_id=$2 ORDER BY created_at DESC LIMIT 50").bind(target).bind(guild_id).fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|r| Infraction {
                id: r.0,
                target_discord_id: r.1,
                kind: r.2,
                reason: r.3,
                issued_by: r.4,
                created_at: r.5,
            })
            .collect())
    }
}
