use crate::{
    discord::bot::DiscordBot,
    services::{balance_service, dota_profile_links, lobby_seed_service, lobby_service},
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
};

pub fn register() -> CreateCommand {
    CreateCommand::new("lobby")
        .description("Show an inhouse lobby")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "number", "Lobby number")
                .required(false),
        )
}

fn player_label(player: &crate::services::whoishere_service::PlayerView) -> String {
    if player
        .discord_id
        .starts_with(lobby_seed_service::SYNTHETIC_PREFIX)
    {
        player.display_name.clone()
    } else {
        format!("<@{}>", player.discord_id)
    }
}

pub async fn run(ctx: &Context, c: &CommandInteraction, bot: &DiscordBot) -> AppResult<()> {
    run_with_balance(ctx, c, bot, false).await
}

pub(crate) async fn run_balanced(
    ctx: &Context,
    c: &CommandInteraction,
    bot: &DiscordBot,
) -> AppResult<()> {
    run_with_balance(ctx, c, bot, true).await
}

async fn run_with_balance(
    ctx: &Context,
    c: &CommandInteraction,
    bot: &DiscordBot,
    balance: bool,
) -> AppResult<()> {
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
        });
    let (lobby, players) = lobby_service::list(
        &bot.pool,
        guild.get() as i64,
        number,
        &c.user.id.to_string(),
    )
    .await?;
    let mut embed = CreateEmbed::new().title(format!(
        "Lobby #{} ({} / 10 players)",
        lobby.number,
        players.len()
    ));
    for player in &players {
        let links = if let Some(account_id) = player.account_id {
            dota_profile_links::markdown_line(account_id)
        } else {
            let dummy_code = lobby_seed_service::dummy_friend_code(&player.discord_id);
            format!(
                "{} · `{dummy_code}`",
                dota_profile_links::markdown_home_line()
            )
        };
        embed = embed.field(
            format!(
                "{} {}",
                bot.rank_emojis.label(player.rank),
                player_label(player)
            ),
            format!(
                "{}-{} ({} games)\n{links}",
                player.wins, player.losses, player.games
            ),
            false,
        );
    }
    if balance {
        let teams = balance_service::split(&players);
        let radiant = teams
            .first
            .iter()
            .map(player_label)
            .collect::<Vec<_>>()
            .join(" ");
        let dire = teams
            .second
            .iter()
            .map(player_label)
            .collect::<Vec<_>>()
            .join(" ");
        embed = embed
            .field("Radiant", radiant, false)
            .field("Dire", dire, false);
    }
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed)),
    )
    .await
    .map_err(|error| AppError::Discord(error.to_string()))?;
    Ok(())
}
