mod config;
mod database;
mod discord;
mod external;
mod models;
mod services;
mod utils;

use anyhow::Result;
use config::Settings;
use database::connect;
use discord::bot::DiscordBot;
use services::rank_lookup_service::RankLookupService;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let settings = Settings::from_env()?;
    tracing_subscriber::fmt()
        .with_env_filter(&settings.log_level)
        .init();

    let pool = connect(&settings.database_url).await?;
    sqlx::migrate!().run(&pool).await?;
    let rank_service = RankLookupService::new(
        pool.clone(),
        settings.opendota_api_key.clone(),
        settings.opendota_requests_per_minute,
    );
    let bot = DiscordBot::new(settings, pool, rank_service);
    info!("Starting Housekeeper");
    bot.start().await?;
    Ok(())
}
