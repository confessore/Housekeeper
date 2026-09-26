use crate::discord::{bot::DiscordBot, commands};
use serenity::{
    all::{Context, EditInteractionResponse, EventHandler, Guild, Interaction, Member, Ready},
    async_trait,
};
pub struct Handler {
    pub bot: DiscordBot,
}
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        if let Err(e) = self.bot.register(&ctx, &ready).await {
            tracing::error!(?e, "command registration failed");
        }
        for guild in ready.guilds {
            if let Err(e) =
                crate::services::guild_role_sync::sync(&self.bot.pool, &ctx, guild.id).await
            {
                tracing::error!(guild_id=%guild.id.get(),?e,"guild role sync failed");
            }
        }
    }
    async fn guild_create(&self, ctx: Context, guild: Guild, _is_new: Option<bool>) {
        if let Err(e) = crate::services::guild_role_sync::sync(&self.bot.pool, &ctx, guild.id).await
        {
            tracing::error!(guild_id=%guild.id.get(),?e,"new guild role sync failed");
        }
    }
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let result = match command.data.name.as_str() {
                "help" => commands::help::run(&ctx, &command, &self.bot).await,
                "whoishere" => commands::whoishere::run(&ctx, &command, &self.bot).await,
                "link-steam" => commands::link_steam::run(&ctx, &command, &self.bot).await,
                "lobby" => commands::lobby::run(&ctx, &command, &self.bot).await,
                "lobby-balance" => commands::lobby_balance::run(&ctx, &command, &self.bot).await,
                "lobby-create" => commands::lobby_create::run(&ctx, &command, &self.bot).await,
                "lobby-list" => commands::lobby_list::run(&ctx, &command, &self.bot).await,
                "lobby-close" => commands::lobby_close::run(&ctx, &command, &self.bot).await,
                "lobby-join" => commands::lobby_join::run(&ctx, &command, &self.bot).await,
                "lobby-leave" => commands::lobby_leave::run(&ctx, &command, &self.bot).await,
                "lobby-add" => commands::lobby_add::run(&ctx, &command, &self.bot).await,
                "lobby-remove" => commands::lobby_remove::run(&ctx, &command, &self.bot).await,
                "lobby-seed" => commands::lobby_seed::run(&ctx, &command, &self.bot).await,
                "admin-role-map" => commands::admin_role_map::run(&ctx, &command, &self.bot).await,
                "admin-role-tier" => {
                    commands::admin_role_tier::run(&ctx, &command, &self.bot).await
                }
                "warn" => commands::warn::run(&ctx, &command, &self.bot).await,
                "ban" => commands::ban::run(&ctx, &command, &self.bot).await,
                "unban" => commands::unban::run(&ctx, &command, &self.bot).await,
                "history" => commands::history::run(&ctx, &command, &self.bot).await,
                _ => Ok(()),
            };
            if let Err(e) = result {
                let response = if command.data.name == "lobby-create" {
                    command
                        .edit_response(
                            &ctx.http,
                            EditInteractionResponse::new().content(format!("Error: {e}")),
                        )
                        .await
                        .map(|_| ())
                } else {
                    command
                        .create_response(
                            &ctx.http,
                            serenity::all::CreateInteractionResponse::Message(
                                serenity::all::CreateInteractionResponseMessage::new()
                                    .content(format!("Error: {e}"))
                                    .ephemeral(true),
                            ),
                        )
                        .await
                };
                if let Err(response_error) = response {
                    tracing::error!(?response_error, "failed to send interaction error response");
                }
            }
        }
    }
    async fn guild_member_update(
        &self,
        _ctx: Context,
        _old: Option<Member>,
        new: Option<Member>,
        _event: serenity::all::GuildMemberUpdateEvent,
    ) {
        if let Some(member) = new {
            let ids = member
                .roles
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            if let Err(error) = crate::services::guild_role_sync::sync_member(
                &self.bot.pool,
                &member.user.id.to_string(),
                &ids,
            )
            .await
            {
                tracing::error!(?error, "member role sync failed");
            }
        }
    }
}
