use crate::{
    database::UserRepository,
    discord::bot::DiscordBot,
    services::{balance_service, dota_profile_links, lobby_seed_service, whoishere_service},
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
    GuildId,
};

pub fn register() -> CreateCommand {
    CreateCommand::new("whoishere")
        .description("Show the players in your voice channel")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Boolean,
                "balance",
                "Split players into balanced teams",
            )
            .required(false),
        )
}

async fn load_voice_members(
    ctx: &Context,
    guild: GuildId,
) -> AppResult<Vec<whoishere_service::VoiceMember>> {
    if let Some(data) = ctx.cache.guild(guild) {
        return Ok(data
            .members
            .values()
            .map(|member| whoishere_service::VoiceMember {
                discord_id: member.user.id.to_string(),
                username: member.display_name().to_owned(),
                bot: member.user.bot,
                channel_id: data
                    .voice_states
                    .get(&member.user.id)
                    .and_then(|state| state.channel_id)
                    .map(|channel| channel.get()),
            })
            .collect());
    }

    tracing::warn!(
        guild_id = guild.get(),
        "guild cache unavailable; resolving voice members through Discord HTTP"
    );
    let members = guild
        .members(&ctx.http, Some(1_000), None)
        .await
        .map_err(|error| AppError::Discord(error.to_string()))?;
    let mut voice_members = Vec::with_capacity(members.len());
    for member in members {
        let channel_id = guild
            .get_user_voice_state(&ctx.http, member.user.id)
            .await
            .ok()
            .and_then(|state| state.channel_id)
            .map(|channel| channel.get());
        voice_members.push(whoishere_service::VoiceMember {
            discord_id: member.user.id.to_string(),
            username: member.display_name().to_owned(),
            bot: member.user.bot,
            channel_id,
        });
    }
    Ok(voice_members)
}

pub async fn build_embed(
    ctx: &Context,
    guild: GuildId,
    requester: &str,
    bot: &DiscordBot,
    balance: bool,
) -> AppResult<CreateEmbed> {
    let voice_members = load_voice_members(ctx, guild).await?;
    let members = whoishere_service::members_in_channel(requester, &voice_members);
    if members.is_empty() {
        return Err(AppError::InvalidInput("join a voice channel first".into()));
    }
    let repo = UserRepository::new(bot.pool.clone());
    let mut players = Vec::new();
    for (discord_id, name) in members {
        let user = repo.find_or_create(&discord_id, &name).await?;
        if !user.inhouse_banned {
            players.push(whoishere_service::view(user));
        }
    }
    whoishere_service::sort_players(&mut players);
    const MAX_PLAYER_FIELDS: usize = 20;
    let mut embed = CreateEmbed::new().title(format!("Who is here? ({} players)", players.len()));
    for player in players.iter().take(MAX_PLAYER_FIELDS) {
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
                player.display_name
            ),
            format!(
                "<@{}>\n{}-{} ({} games)\n{links}",
                player.discord_id, player.wins, player.losses, player.games
            ),
            true,
        );
    }
    if players.len() > MAX_PLAYER_FIELDS {
        embed = embed.field(
            "…",
            format!(
                "+{} more players not shown",
                players.len() - MAX_PLAYER_FIELDS
            ),
            false,
        );
    }
    if balance {
        let teams = balance_service::split(&players);
        let truncate_team = |team: Vec<String>| {
            let team = team.join(" ");
            if team.chars().count() <= 1_024 {
                team
            } else {
                format!("{}…", team.chars().take(1_023).collect::<String>())
            }
        };
        embed = embed
            .field(
                "Radiant",
                truncate_team(
                    teams
                        .first
                        .iter()
                        .map(|player| format!("<@{}>", player.discord_id))
                        .collect(),
                ),
                false,
            )
            .field(
                "Dire",
                truncate_team(
                    teams
                        .second
                        .iter()
                        .map(|player| format!("<@{}>", player.discord_id))
                        .collect(),
                ),
                false,
            );
    }
    Ok(embed)
}

pub async fn run(ctx: &Context, c: &CommandInteraction, bot: &DiscordBot) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    let balance = c
        .data
        .options
        .iter()
        .find(|o| o.name == "balance")
        .and_then(|o| match o.value {
            CommandDataOptionValue::Boolean(v) => Some(v),
            _ => None,
        })
        .unwrap_or(false);
    let embed = build_embed(ctx, guild, &c.user.id.to_string(), bot, balance).await?;
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .embed(embed)
                .components(crate::discord::components::whoishere_buttons(
                    &c.user.id.to_string(),
                    balance,
                )),
        ),
    )
    .await
    .map_err(|e| AppError::Discord(e.to_string()))?;
    if let Ok(message) = c.get_response(&ctx.http).await {
        bot.panel_registry.remember(
            crate::discord::panel_registry::PanelKey {
                scope: "whoishere".into(),
                key: c.user.id.to_string(),
                channel_id: c.channel_id,
            },
            message.id,
        );
    }
    Ok(())
}
