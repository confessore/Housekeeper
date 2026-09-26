use crate::utils::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct Settings {
    pub discord_token: String,
    pub database_url: String,
    pub opendota_api_key: Option<String>,
    pub opendota_requests_per_minute: u32,
    pub log_level: String,
    pub developer_discord_ids: Vec<String>,
    pub command_guild_id: Option<u64>,
}

impl Settings {
    pub fn from_env() -> AppResult<Self> {
        if let Err(error) = dotenvy::dotenv() {
            if !matches!(
                error,
                dotenvy::Error::Io(ref io_error)
                    if io_error.kind() == std::io::ErrorKind::NotFound
            ) {
                return Err(AppError::Config(format!("failed to load .env: {error}")));
            }
        }
        let required = |name: &str| {
            std::env::var(name).map_err(|_| AppError::Config(format!("{name} is required")))
        };
        let developer_discord_ids = std::env::var("DEVELOPER_DISCORD_IDS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        let command_guild_id = std::env::var("DISCORD_COMMAND_GUILD_ID")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(|value| {
                value.parse::<u64>().map_err(|_| {
                    AppError::Config("DISCORD_COMMAND_GUILD_ID must be a number".into())
                })
            })
            .transpose()?;
        Ok(Self {
            discord_token: required("DISCORD_TOKEN")?,
            database_url: required("DATABASE_URL")?,
            opendota_api_key: std::env::var("OPENDOTA_API_KEY")
                .ok()
                .filter(|s| !s.is_empty()),
            opendota_requests_per_minute: std::env::var("OPENDOTA_REQUESTS_PER_MINUTE")
                .ok()
                .and_then(|value| value.parse().ok())
                .filter(|value| *value > 0)
                .unwrap_or(60),
            log_level: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
            developer_discord_ids,
            command_guild_id,
        })
    }
}
