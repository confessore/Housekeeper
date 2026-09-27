use crate::{
    discord::bot::DiscordBot,
    services::lobby_service,
    utils::error::{AppError, AppResult},
};
use serenity::all::{CommandInteraction, Context, CreateCommand};

pub fn register() -> CreateCommand {
    CreateCommand::new("lobby-leave").description("Leave your current inhouse lobby")
}

pub async fn run(ctx: &Context, c: &CommandInteraction, bot: &DiscordBot) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    let (lobby, _) =
        lobby_service::list(&bot.pool, guild.get() as i64, None, &c.user.id.to_string()).await?;
    lobby_service::leave(&bot.pool, guild.get() as i64, &c.user.id.to_string()).await?;
    crate::discord::components::send_lobby_panel(ctx, c, bot, lobby.number, lobby.id, false).await
}
