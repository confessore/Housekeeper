mod lobby_repository;
mod repositories;
use crate::utils::error::AppResult;
pub use lobby_repository::LobbyRepository;
pub use repositories::*;
use sqlx::PgPool;
pub async fn connect(url: &str) -> AppResult<PgPool> {
    Ok(sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(url)
        .await?)
}
