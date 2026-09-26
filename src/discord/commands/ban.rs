use super::moderation_common::run_infraction;
use crate::{discord::bot::DiscordBot, models::InfractionKind, utils::error::AppResult};
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
};
pub fn register() -> CreateCommand {
    CreateCommand::new("ban")
        .description("Ban a player from inhouses")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Player").required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "reason", "Reason").required(true),
        )
}
pub async fn run(ctx: &Context, c: &CommandInteraction, b: &DiscordBot) -> AppResult<()> {
    run_infraction(ctx, c, b, InfractionKind::Ban).await
}
