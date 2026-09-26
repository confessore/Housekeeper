use crate::{
    discord::bot::DiscordBot,
    services::moderation::infractions,
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
pub fn register() -> CreateCommand {
    CreateCommand::new("history")
        .description("View a player's inhouse infraction history")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Player").required(true),
        )
}
pub async fn run(ctx: &Context, c: &CommandInteraction, b: &DiscordBot) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    let target = c
        .data
        .options
        .iter()
        .find(|o| o.name == "user")
        .and_then(|o| match o.value {
            CommandDataOptionValue::User(id) => Some(id.to_string()),
            _ => None,
        })
        .ok_or_else(|| AppError::InvalidInput("user required".into()))?;
    let rows =
        infractions::history(&b.pool, guild.get() as i64, &c.user.id.to_string(), &target).await?;
    let text = if rows.is_empty() {
        "No infraction history.".into()
    } else {
        rows.into_iter()
            .map(|r| {
                format!(
                    "#{} {} — {} — {} (by <@{}>, target <@{}>)",
                    r.id,
                    r.created_at.format("%Y-%m-%d"),
                    r.kind,
                    r.reason,
                    r.issued_by,
                    r.target_discord_id
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(text)
                .ephemeral(true),
        ),
    )
    .await
    .map_err(|e| AppError::Discord(e.to_string()))?;
    Ok(())
}
