use crate::{database::RoleRepository, models::RoleTier, utils::error::AppResult};
use sqlx::PgPool;

pub async fn resolve(pool: &PgPool, guild_id: i64, user_id: &str) -> AppResult<Option<RoleTier>> {
    RoleRepository::new(pool.clone())
        .resolve(guild_id, user_id)
        .await
}
