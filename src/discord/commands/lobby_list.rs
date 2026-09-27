use crate::{discord::bot::DiscordBot, utils::error::AppResult};
use serenity::all::{CommandInteraction, Context, CreateCommand};

pub fn register() -> CreateCommand {
    CreateCommand::new("lobby-list").description("List active inhouse lobbies")
}

pub async fn run(ctx: &Context, c: &CommandInteraction, bot: &DiscordBot) -> AppResult<()> {
    crate::discord::components::send_lobby_list(ctx, c, bot).await
}
