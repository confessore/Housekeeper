use crate::{discord::bot::DiscordBot, utils::error::AppResult};
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
};

pub fn register() -> CreateCommand {
    CreateCommand::new("lobby-balance")
        .description("Show balanced teams for an inhouse lobby")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "number", "Lobby number")
                .required(true),
        )
}

pub async fn run(ctx: &Context, c: &CommandInteraction, bot: &DiscordBot) -> AppResult<()> {
    super::lobby::run_balanced(ctx, c, bot).await
}
