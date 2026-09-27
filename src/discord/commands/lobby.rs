use crate::{
    discord::bot::DiscordBot,
    services::{balance_service, dota_profile_links, lobby_seed_service, lobby_service},
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
    GuildId,
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

fn server_display_name(
    ctx: &Context,
    guild: GuildId,
    player: &crate::services::whoishere_service::PlayerView,
) -> String {
    let Some(id) = player.discord_id.parse::<u64>().ok() else {
        return player.display_name.clone();
    };
    let Some(data) = ctx.cache.guild(guild) else {
        return player.display_name.clone();
    };
    data.members
        .get(&serenity::all::UserId::new(id))
        .map(|member| {
            member
                .nick
                .clone()
                .unwrap_or_else(|| member.user.name.clone())
        })
        .unwrap_or_else(|| player.display_name.clone())
}

pub(crate) async fn build_embed(
    ctx: &Context,
    bot: &DiscordBot,
    guild: GuildId,
    number: Option<i32>,
    requester: &str,
    balance: bool,
) -> AppResult<CreateEmbed> {
    let (lobby, players) =
        lobby_service::list(&bot.pool, guild.get() as i64, number, requester).await?;
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
                server_display_name(ctx, guild, player)
            ),
            format!(
                "{}\n{}-{} ({} games)\n{links}",
                player_label(player),
                player.wins,
                player.losses,
                player.games
            ),
            true,
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
    Ok(embed)
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
    let lobby = match number {
        Some(number) => lobby_service::resolve(&bot.pool, guild.get() as i64, number).await?,
        None => {
            lobby_service::list(&bot.pool, guild.get() as i64, None, &c.user.id.to_string())
                .await?
                .0
        }
    };
    let embed = build_embed(
        ctx,
        bot,
        guild,
        Some(lobby.number),
        &c.user.id.to_string(),
        balance,
    )
    .await?;
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .embed(embed)
                .components(crate::discord::components::lobby_buttons(
                    lobby.number,
                    lobby.id,
                    balance,
                )),
        ),
    )
    .await
    .map_err(|error| AppError::Discord(error.to_string()))?;
    if let Ok(message) = c.get_response(&ctx.http).await {
        bot.panel_registry.remember(
            crate::discord::panel_registry::PanelKey {
                scope: "lobby".into(),
                key: lobby.id.to_string(),
                channel_id: c.channel_id,
            },
            message.id,
        );
    }
    Ok(())
}
