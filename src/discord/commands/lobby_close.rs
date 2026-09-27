use super::moderation_common::require_role_manager;
use crate::{
    discord::{bot::DiscordBot, panel_registry::PanelKey},
    services::lobby_service,
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

pub fn register() -> CreateCommand {
    CreateCommand::new("lobby-close")
        .description("Close an inhouse lobby")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "number", "Lobby number")
                .required(true),
        )
}

pub async fn run(ctx: &Context, c: &CommandInteraction, bot: &DiscordBot) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    require_role_manager(c, bot, guild.get() as i64).await?;
    let number = c
        .data
        .options
        .iter()
        .find(|option| option.name == "number")
        .and_then(|option| match option.value {
            serenity::all::CommandDataOptionValue::Integer(value) => i32::try_from(value).ok(),
            _ => None,
        })
        .ok_or_else(|| AppError::InvalidInput("lobby number is required".into()))?;
    let lobby = lobby_service::resolve_for_guild(&bot.pool, guild.get() as i64, number).await?;
    lobby_service::close(&bot.pool, guild.get() as i64, number).await?;
    bot.panel_registry
        .evict(
            &ctx.http,
            &PanelKey {
                scope: "lobby".into(),
                key: lobby.id.to_string(),
                channel_id: c.channel_id,
            },
        )
        .await;
    let (embed, components) = crate::discord::components::ended(number);
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .embed(embed)
                .components(components),
        ),
    )
    .await
    .map_err(|error| AppError::Discord(error.to_string()))?;
    Ok(())
}
