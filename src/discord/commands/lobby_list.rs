use crate::{
    discord::bot::DiscordBot,
    services::lobby_service,
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

pub fn register() -> CreateCommand {
    CreateCommand::new("lobby-list").description("List active inhouse lobbies")
}

pub async fn run(ctx: &Context, c: &CommandInteraction, bot: &DiscordBot) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    let lobbies = lobby_service::active(&bot.pool, guild.get() as i64).await?;
    let text = if lobbies.is_empty() {
        "There are no active lobbies.".into()
    } else {
        let mut text = String::from("**Active lobbies**\n");
        for (lobby, count) in lobbies {
            text.push_str(&format!(
                "• **Lobby #{}** — {}/10 players, {} minutes remaining\n",
                lobby.number,
                count,
                lobby.minutes_remaining()
            ));
        }
        text
    };
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(text)),
    )
    .await
    .map_err(|error| AppError::Discord(error.to_string()))?;
    Ok(())
}
