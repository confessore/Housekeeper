use crate::{
    discord::bot::DiscordBot,
    services::{balance_service, lobby_service},
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
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
    if player.discord_id.starts_with("housekeeper-test-") {
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
    let mut text = format!(
        "**Lobby #{}** ({} / 10 players)\n",
        lobby.number,
        players.len()
    );
    for player in &players {
        let label = player_label(player);
        text.push_str(&format!(
            "• {label} — {} | {}-{} ({} games)\n",
            player.rank, player.wins, player.losses, player.games
        ));
    }
    if balance {
        let teams = balance_service::split(&players);
        text.push_str("\n**Radiant**\n");
        for player in teams.first {
            text.push_str(&format!("{} ", player_label(&player)));
        }
        text.push_str("\n**Dire**\n");
        for player in teams.second {
            text.push_str(&format!("{} ", player_label(&player)));
        }
    }
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(text)),
    )
    .await
    .map_err(|error| AppError::Discord(error.to_string()))?;
    Ok(())
}
