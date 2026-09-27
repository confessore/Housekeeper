use crate::{
    database::UserRepository,
    discord::bot::DiscordBot,
    services::{balance_service, dota_profile_links, role_resolver, whoishere_service},
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
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
    let members = {
        let data = ctx
            .cache
            .guild(guild)
            .ok_or_else(|| AppError::Discord("guild cache unavailable".into()))?;
        let voice_members = data
            .members
            .values()
            .map(|member| whoishere_service::VoiceMember {
                discord_id: member.user.id.to_string(),
                username: member.user.name.clone(),
                bot: member.user.bot,
                channel_id: data
                    .voice_states
                    .get(&member.user.id)
                    .and_then(|state| state.channel_id)
                    .map(|channel| channel.get()),
            })
            .collect::<Vec<_>>();
        let members = whoishere_service::members_in_channel(&c.user.id.to_string(), &voice_members);
        if members.is_empty() {
            return Err(AppError::InvalidInput("join a voice channel first".into()));
        }
        members
    };
    let repo = UserRepository::new(bot.pool.clone());
    let mut players = Vec::new();
    for (discord_id, name) in members {
        let user = repo.find_or_create(&discord_id, &name).await?;
        if !user.inhouse_banned {
            let role =
                role_resolver::resolve(&bot.pool, guild.get() as i64, &user.discord_id).await?;
            players.push(whoishere_service::view(user, role));
        }
    }
    whoishere_service::sort_players(&mut players);
    let mut text = format!("**Who is here?** ({} players)\n", players.len());
    for p in &players {
        let role = p
            .role
            .as_ref()
            .map(|role| role.name.as_str())
            .unwrap_or("Member");
        text.push_str(&format!(
            "• <@{}> — {} | {} | {}-{} ({} games)\n",
            p.discord_id,
            bot.rank_emojis.label(p.rank),
            role,
            p.wins,
            p.losses,
            p.games
        ));
        if let Some(account_id) = p.account_id {
            text.push_str(&format!(
                "  {}\n",
                dota_profile_links::markdown_line(account_id)
            ));
        }
    }
    if balance {
        let teams = balance_service::split(&players);
        text.push_str("\n**Radiant**\n");
        for p in teams.first {
            text.push_str(&format!("<@{}> ", p.discord_id));
        }
        text.push_str("\n**Dire**\n");
        for p in teams.second {
            text.push_str(&format!("<@{}> ", p.discord_id));
        }
    }
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(text)),
    )
    .await
    .map_err(|e| AppError::Discord(e.to_string()))?;
    Ok(())
}
