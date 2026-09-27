use crate::{
    discord::bot::DiscordBot,
    services::lobby_service,
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, EditInteractionResponse,
};

pub fn register() -> CreateCommand {
    CreateCommand::new("lobby-create")
        .description("Create an inhouse lobby")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "hours",
                "Lifetime in hours (1-24)",
            )
            .required(false),
        )
}

pub async fn run(ctx: &Context, c: &CommandInteraction, bot: &DiscordBot) -> AppResult<()> {
    c.defer(ctx)
        .await
        .map_err(|error| AppError::Discord(error.to_string()))?;
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    let hours = c
        .data
        .options
        .iter()
        .find(|option| option.name == "hours")
        .and_then(|option| match option.value {
            CommandDataOptionValue::Integer(value) => Some(value),
            _ => None,
        });
    let lobby =
        lobby_service::create(&bot.pool, guild.get() as i64, &c.user.id.to_string(), hours).await?;
    let embed = crate::discord::commands::lobby::build_embed(
        ctx,
        bot,
        guild,
        Some(lobby.number),
        &c.user.id.to_string(),
        false,
    )
    .await?;
    let key = crate::discord::panel_registry::PanelKey::lobby(lobby.number, c.channel_id);
    bot.panel_registry.evict(&ctx.http, &key).await;
    let message = c
        .edit_response(
            ctx,
            EditInteractionResponse::new().embed(embed).components(
                crate::discord::components::lobby_buttons(lobby.number, lobby.id, false),
            ),
        )
        .await
        .map_err(|error| AppError::Discord(error.to_string()))?;
    bot.panel_registry.remember(key, message.id).await;
    Ok(())
}
