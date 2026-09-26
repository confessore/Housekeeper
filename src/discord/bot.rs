use super::handler::Handler;
use crate::{config::Settings, services::rank_lookup_service::RankLookupService};
use serenity::all::{Client, Context, GatewayIntents, GuildId, Ready};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
pub struct DiscordBot {
    pub settings: Arc<Settings>,
    pub pool: PgPool,
    pub rank_service: Arc<RankLookupService>,
}
impl DiscordBot {
    pub fn new(settings: Settings, pool: PgPool, rank_service: Arc<RankLookupService>) -> Self {
        Self {
            settings: Arc::new(settings),
            pool,
            rank_service,
        }
    }
    pub async fn start(&self) -> anyhow::Result<()> {
        let intents = GatewayIntents::GUILDS
            | GatewayIntents::GUILD_MEMBERS
            | GatewayIntents::GUILD_VOICE_STATES;
        let mut client = Client::builder(&self.settings.discord_token, intents)
            .event_handler(Handler { bot: self.clone() })
            .await?;
        client.start().await?;
        Ok(())
    }
    pub async fn register(&self, ctx: &Context, ready: &Ready) -> anyhow::Result<()> {
        serenity::all::Command::set_global_commands(&ctx.http, crate::discord::commands::all())
            .await?;
        tracing::info!("Registered global commands for {}", ready.user.name);
        if let Some(guild_id) = self.settings.command_guild_id {
            GuildId::new(guild_id)
                .set_commands(&ctx.http, crate::discord::commands::all())
                .await?;
            tracing::info!(
                guild_id,
                "Registered development guild commands for {}",
                ready.user.name
            );
        }
        Ok(())
    }
}
