use super::moderation_common::require_role_manager;
use crate::{
    database::RoleRepository,
    discord::bot::DiscordBot,
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};

pub fn register() -> CreateCommand {
    CreateCommand::new("admin-role-map")
        .description("Map a Discord role to a custom guild tier")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Role, "discord_role", "Discord role")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "tier", "Configured tier name")
                .required(true),
        )
}

pub async fn run(ctx: &Context, c: &CommandInteraction, bot: &DiscordBot) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    require_role_manager(c, bot, guild.get() as i64).await?;
    let role_id = c
        .data
        .options
        .iter()
        .find(|o| o.name == "discord_role")
        .and_then(|o| match o.value {
            CommandDataOptionValue::Role(id) => Some(id.to_string()),
            _ => None,
        })
        .ok_or_else(|| AppError::InvalidInput("discord_role is required".into()))?;
    let tier_name = c
        .data
        .options
        .iter()
        .find(|o| o.name == "tier")
        .and_then(|o| o.value.as_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| AppError::InvalidInput("tier is required".into()))?;
    sqlx::query("INSERT INTO guilds(discord_guild_id,name) VALUES($1,$2) ON CONFLICT(discord_guild_id) DO NOTHING")
        .bind(guild.get() as i64)
        .bind("Discord guild")
        .execute(&bot.pool)
        .await?;
    let internal_guild_id: i64 =
        sqlx::query_scalar("SELECT id FROM guilds WHERE discord_guild_id=$1")
            .bind(guild.get() as i64)
            .fetch_one(&bot.pool)
            .await?;
    let repo = RoleRepository::new(bot.pool.clone());
    let tier = repo
        .find_tier_by_name(internal_guild_id, tier_name)
        .await?
        .ok_or_else(|| {
            AppError::InvalidInput(format!(
                "tier `{tier_name}` does not exist; create it first with /admin-role-tier"
            ))
        })?;
    repo.map_role(internal_guild_id, &role_id, tier.id).await?;
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(format!("Discord role mapped to `{}`.", tier.name))
                .ephemeral(true),
        ),
    )
    .await
    .map_err(|e| AppError::Discord(e.to_string()))?;
    Ok(())
}
