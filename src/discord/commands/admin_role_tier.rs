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
    CreateCommand::new("admin-role-tier")
        .description("Manage this guild's custom role tiers")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "create",
                "Create a role tier",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "name", "Tier name")
                    .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "moderator",
                    "Can this tier manage the bot?",
                )
                .required(true),
            ),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "list",
            "List this guild's role tiers",
        ))
}

pub async fn run(ctx: &Context, c: &CommandInteraction, bot: &DiscordBot) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    require_role_manager(c, bot, guild.get() as i64).await?;
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
    let subcommand = c
        .data
        .options
        .first()
        .ok_or_else(|| AppError::InvalidInput("subcommand is required".into()))?;
    let repo = RoleRepository::new(bot.pool.clone());
    let subcommand_options = match &subcommand.value {
        CommandDataOptionValue::SubCommand(options) => options,
        _ => return Err(AppError::InvalidInput("invalid subcommand".into())),
    };
    let content = match subcommand.name.as_str() {
        "create" => {
            let name = subcommand_options
                .iter()
                .find(|o| o.name == "name")
                .and_then(|o| match &o.value {
                    CommandDataOptionValue::String(value) => Some(value.trim()),
                    _ => None,
                })
                .filter(|name| !name.is_empty())
                .ok_or_else(|| AppError::InvalidInput("name is required".into()))?;
            let moderator = subcommand_options
                .iter()
                .find(|o| o.name == "moderator")
                .and_then(|o| match o.value {
                    CommandDataOptionValue::Boolean(value) => Some(value),
                    _ => None,
                })
                .ok_or_else(|| AppError::InvalidInput("moderator is required".into()))?;
            repo.create_tier(internal_guild_id, name, moderator).await?;
            format!("Role tier `{name}` created.")
        }
        "list" => {
            let tiers = repo.list_tiers(internal_guild_id).await?;
            if tiers.is_empty() {
                "No role tiers configured.".into()
            } else {
                tiers
                    .into_iter()
                    .map(|tier| {
                        format!(
                            "• {} ({})",
                            tier.name,
                            if tier.is_moderator {
                                "manager"
                            } else {
                                "member"
                            }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
        _ => return Err(AppError::InvalidInput("unknown subcommand".into())),
    };
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(content)
                .ephemeral(true),
        ),
    )
    .await
    .map_err(|e| AppError::Discord(e.to_string()))?;
    Ok(())
}
