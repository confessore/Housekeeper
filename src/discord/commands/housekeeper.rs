use crate::{discord::bot::DiscordBot, utils::error::AppResult};
use serenity::all::{CommandInteraction, Context, CreateCommand};

pub fn register() -> CreateCommand {
    CreateCommand::new("housekeeper").description("Open the Housekeeper control panel")
}

pub async fn run(ctx: &Context, c: &CommandInteraction, _bot: &DiscordBot) -> AppResult<()> {
    super::super::components::send_hub(ctx, c).await
}
