use crate::{
    database::UserRepository,
    discord::{bot::DiscordBot, member_name},
    services::lobby_service,
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption,
};

pub fn register() -> CreateCommand {
    CreateCommand::new("lobby-join")
        .description("Join an inhouse lobby")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "number", "Lobby number")
                .required(true),
        )
}

pub async fn run(ctx: &Context, c: &CommandInteraction, bot: &DiscordBot) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    let number = c
        .data
        .options
        .iter()
        .find(|option| option.name == "number")
        .and_then(|option| match option.value {
            CommandDataOptionValue::Integer(value) => i32::try_from(value).ok(),
            _ => None,
        })
        .ok_or_else(|| AppError::InvalidInput("lobby number is required".into()))?;
    let display_name = member_name::interaction_name(&c.user, c.member.as_deref());
    let user = UserRepository::new(bot.pool.clone())
        .find_or_create(&c.user.id.to_string(), &display_name)
        .await?;
    lobby_service::join(
        &bot.pool,
        guild.get() as i64,
        number,
        &user,
        &bot.rank_service,
    )
    .await?;
    let lobby = lobby_service::resolve_for_guild(&bot.pool, guild.get() as i64, number).await?;
    crate::discord::components::send_lobby_panel(ctx, c, bot, lobby.number, lobby.id, false).await
}
