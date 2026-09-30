use crate::{
    database::UserRepository,
    discord::{bot::DiscordBot, commands, member_name, panel_registry::PanelKey},
    services::{lobby_service, register_service},
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    ActionRowComponent, ButtonStyle, CommandInteraction, ComponentInteraction,
    ComponentInteractionDataKind, Context, CreateActionRow, CreateButton, CreateEmbed,
    CreateInputText, CreateInteractionResponse, CreateInteractionResponseMessage, CreateModal,
    CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption, EditInteractionResponse,
    InputTextStyle, ModalInteraction,
};

const PREFIX: &str = "hk";

fn panel_id(number: i32, id: i64, action: &str) -> String {
    format!("{PREFIX}:l:{number}:{id}:{action}")
}

fn parse_panel_id(value: &str) -> Option<(i32, i64, &str)> {
    let mut parts = value.split(':');
    (parts.next() == Some(PREFIX) && parts.next() == Some("l"))
        .then(|| {
            Some((
                parts.next()?.parse().ok()?,
                parts.next()?.parse().ok()?,
                parts.next()?,
            ))
        })
        .flatten()
}

fn parse_lobby_steam_modal_id(value: &str) -> Option<(i32, i64)> {
    let mut parts = value.split(':');
    if parts.next() != Some(PREFIX) || parts.next() != Some("steam_lobby") {
        return None;
    }
    let number = parts.next()?.parse().ok()?;
    let id = parts.next()?.parse().ok()?;
    (number > 0 && id > 0 && parts.next().is_none()).then_some((number, id))
}

pub(crate) fn lobby_steam_modal(number: i32, id: i64) -> CreateModal {
    CreateModal::new(
        format!("{PREFIX}:steam_lobby:{number}:{id}"),
        "Link Steam account",
    )
    .components(vec![CreateActionRow::InputText(
        CreateInputText::new(InputTextStyle::Short, "Steam friend code", "friend_code")
            .placeholder("digits, optionally with dashes"),
    )])
}

fn parse_whoishere_id(value: &str) -> Option<(&str, &str)> {
    let mut parts = value.split(':');
    (parts.next() == Some(PREFIX) && parts.next() == Some("w"))
        .then(|| Some((parts.next()?, parts.next()?)))
        .flatten()
}

pub(crate) fn lobby_buttons(number: i32, id: i64, balanced: bool) -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(panel_id(number, id, "join"))
            .label("Join")
            .style(ButtonStyle::Success),
        CreateButton::new(panel_id(number, id, "leave"))
            .label("Leave")
            .style(ButtonStyle::Secondary),
        CreateButton::new(panel_id(
            number,
            id,
            if balanced { "plain" } else { "balance" },
        ))
        .label(if balanced {
            "Hide teams"
        } else {
            "Balance teams"
        })
        .style(ButtonStyle::Primary),
        CreateButton::new(panel_id(number, id, "refresh"))
            .label("Refresh")
            .style(ButtonStyle::Secondary),
        CreateButton::new(panel_id(number, id, "manage"))
            .label("Manage")
            .style(ButtonStyle::Danger),
    ])]
}

async fn panel(
    ctx: &Context,
    bot: &DiscordBot,
    guild_id: u64,
    number: i32,
    id: i64,
    balanced: bool,
) -> AppResult<(CreateEmbed, Vec<CreateActionRow>)> {
    let embed = commands::lobby::build_embed(
        ctx,
        bot,
        serenity::all::GuildId::new(guild_id),
        Some(number),
        "",
        balanced,
    )
    .await?;
    Ok((embed, lobby_buttons(number, id, balanced)))
}

pub(crate) async fn send_lobby_panel(
    ctx: &Context,
    c: &CommandInteraction,
    bot: &DiscordBot,
    number: i32,
    id: i64,
    balanced: bool,
) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    let key = PanelKey::lobby(number, c.channel_id);
    bot.panel_registry
        .replace(&ctx.http, key, || async {
            let embed = commands::lobby::build_embed(
                ctx,
                bot,
                guild,
                Some(number),
                &c.user.id.to_string(),
                balanced,
            )
            .await?;
            c.create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .components(lobby_buttons(number, id, balanced)),
                ),
            )
            .await
            .map_err(|error| AppError::Discord(error.to_string()))?;
            c.get_response(&ctx.http)
                .await
                .map(|message| message.id)
                .map_err(|error| AppError::Discord(error.to_string()))
        })
        .await
}

pub(crate) fn ended(number: i32) -> (CreateEmbed, Vec<CreateActionRow>) {
    (
        CreateEmbed::new()
            .title(format!("Lobby #{number} has ended"))
            .description("This lobby expired, was closed, or its number has been reused. Open the lobby list to choose an active lobby."),
        vec![CreateActionRow::Buttons(vec![
            CreateButton::new("hk:list").label("Open lobby list").style(ButtonStyle::Primary),
        ])],
    )
}

fn hub_message() -> CreateInteractionResponseMessage {
    CreateInteractionResponseMessage::new()
        .embed(
            CreateEmbed::new()
                .title("Housekeeper")
                .description("Use the buttons below to access the same voice and lobby workflows available through slash commands."),
        )
        .components(vec![
            CreateActionRow::Buttons(vec![
                CreateButton::new("hk:whoishere")
                    .label("Who is here")
                    .style(ButtonStyle::Primary),
                CreateButton::new("hk:list")
                    .label("Lobbies")
                    .style(ButtonStyle::Primary),
                CreateButton::new("hk:current")
                    .label("My lobby")
                    .style(ButtonStyle::Secondary),
                CreateButton::new("hk:create")
                    .label("Create lobby")
                    .style(ButtonStyle::Success),
            ]),
            CreateActionRow::Buttons(vec![
                CreateButton::new("hk:steam")
                    .label("Link Steam")
                    .style(ButtonStyle::Secondary),
                CreateButton::new("hk:help")
                    .label("Help")
                    .style(ButtonStyle::Secondary),
            ]),
        ])
}

pub async fn send_hub(ctx: &Context, c: &CommandInteraction) -> AppResult<()> {
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(hub_message().ephemeral(true)),
    )
    .await
    .map_err(|error| AppError::Discord(error.to_string()))?;
    Ok(())
}

fn lobby_list_view(lobbies: &[(crate::models::Lobby, i64)]) -> (CreateEmbed, Vec<CreateActionRow>) {
    let mut embed = CreateEmbed::new().title("Active lobbies");
    let mut buttons = Vec::new();
    for (lobby, count) in lobbies.iter().take(20) {
        embed = embed.field(
            format!("Lobby #{}", lobby.number),
            format!(
                "{count}/10 players — {} minutes remaining",
                lobby.minutes_remaining()
            ),
            false,
        );
        buttons.push(
            CreateButton::new(format!("hk:open:{}:{}", lobby.number, lobby.id))
                .label(format!("Open #{}", lobby.number))
                .style(ButtonStyle::Primary),
        );
    }
    buttons.extend([
        CreateButton::new("hk:list:refresh")
            .label("Refresh")
            .style(ButtonStyle::Secondary),
        CreateButton::new("hk:create")
            .label("Create lobby")
            .style(ButtonStyle::Success),
        CreateButton::new("hk:home")
            .label("Home")
            .style(ButtonStyle::Secondary),
    ]);
    let mut rows = Vec::new();
    while !buttons.is_empty() && rows.len() < 5 {
        rows.push(CreateActionRow::Buttons(
            buttons.drain(..buttons.len().min(5)).collect(),
        ));
    }
    (
        embed.description(if lobbies.is_empty() {
            "No active lobbies."
        } else {
            "Select a lobby to open its control panel."
        }),
        rows,
    )
}

async fn refresh_lobby_list(
    ctx: &Context,
    c: &ComponentInteraction,
    bot: &DiscordBot,
) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    let lobbies = lobby_service::active(&bot.pool, guild.get() as i64).await?;
    let (embed, components) = lobby_list_view(&lobbies);
    c.create_response(
        ctx,
        CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::new()
                .embed(embed)
                .components(components),
        ),
    )
    .await
    .map_err(|error| AppError::Discord(error.to_string()))
}

pub async fn send_lobby_list(
    ctx: &Context,
    c: &CommandInteraction,
    bot: &DiscordBot,
) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    let lobbies = lobby_service::active(&bot.pool, guild.get() as i64).await?;
    let (embed, action_rows) = lobby_list_view(&lobbies);
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .embed(embed)
                .components(action_rows)
                .ephemeral(true),
        ),
    )
    .await
    .map_err(|error| AppError::Discord(error.to_string()))?;
    Ok(())
}

async fn ephemeral(
    c: &ComponentInteraction,
    ctx: &Context,
    text: impl Into<String>,
) -> AppResult<()> {
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(text)
                .ephemeral(true),
        ),
    )
    .await
    .map_err(|error| AppError::Discord(error.to_string()))?;
    Ok(())
}

async fn publish_component(
    ctx: &Context,
    c: &ComponentInteraction,
    bot: &DiscordBot,
    key: PanelKey,
    embed: CreateEmbed,
    components: Vec<CreateActionRow>,
) -> AppResult<()> {
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("The shared panel is available in the channel.")
                .ephemeral(true),
        ),
    )
    .await
    .map_err(|e| AppError::Discord(e.to_string()))?;
    bot.panel_registry
        .publish(&ctx.http, key, || async { Ok((embed, components)) })
        .await?;
    Ok(())
}

async fn update_panel(
    c: &ComponentInteraction,
    ctx: &Context,
    bot: &DiscordBot,
    number: i32,
    id: i64,
    balanced: bool,
) -> AppResult<()> {
    c.create_response(ctx, CreateInteractionResponse::Acknowledge)
        .await
        .map_err(|e| AppError::Discord(e.to_string()))?;
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    let key = PanelKey::lobby(number, c.channel_id);
    let Some(_) = lobby_service::resolve_panel(&bot.pool, guild.get() as i64, number, id).await?
    else {
        c.create_response(
            ctx,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(ended(number).0)
                    .components(ended(number).1),
            ),
        )
        .await
        .map_err(|e| AppError::Discord(e.to_string()))?;
        return Ok(());
    };
    bot.panel_registry
        .bump(&ctx.http, key, || {
            panel(ctx, bot, guild.get(), number, id, balanced)
        })
        .await?;
    Ok(())
}

pub(crate) fn whoishere_buttons(requester: &str, balanced: bool) -> Vec<CreateActionRow> {
    let toggle_action = if balanced { "plain" } else { "balance" };
    let toggle = CreateButton::new(format!("hk:w:{requester}:{toggle_action}"))
        .label(if balanced {
            "Show roster"
        } else {
            "Balance teams"
        })
        .style(if balanced {
            ButtonStyle::Secondary
        } else {
            ButtonStyle::Primary
        });
    vec![CreateActionRow::Buttons(vec![
        toggle,
        CreateButton::new(format!("hk:w:{requester}:refresh"))
            .label("Refresh")
            .style(ButtonStyle::Secondary),
    ])]
}

async fn send_whoishere(
    ctx: &Context,
    c: &ComponentInteraction,
    bot: &DiscordBot,
    balanced: bool,
) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    let embed =
        commands::whoishere::build_embed(ctx, guild, &c.user.id.to_string(), bot, balanced).await?;
    publish_component(
        ctx,
        c,
        bot,
        PanelKey {
            scope: "whoishere".into(),
            key: c.user.id.to_string(),
            channel_id: c.channel_id,
        },
        embed,
        whoishere_buttons(&c.user.id.to_string(), balanced),
    )
    .await
}

async fn update_whoishere(
    ctx: &Context,
    c: &ComponentInteraction,
    bot: &DiscordBot,
    requester: &str,
    balanced: bool,
) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    let embed = commands::whoishere::build_embed(ctx, guild, requester, bot, balanced).await?;
    c.create_response(
        ctx,
        CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::new()
                .embed(embed)
                .components(whoishere_buttons(requester, balanced)),
        ),
    )
    .await
    .map_err(|e| AppError::Discord(e.to_string()))?;
    Ok(())
}

async fn send_current_lobby(
    ctx: &Context,
    c: &ComponentInteraction,
    bot: &DiscordBot,
) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    let (lobby, _) =
        lobby_service::list(&bot.pool, guild.get() as i64, None, &c.user.id.to_string()).await?;
    let (embed, components) = panel(ctx, bot, guild.get(), lobby.number, lobby.id, false).await?;
    publish_component(
        ctx,
        c,
        bot,
        PanelKey::lobby(lobby.number, c.channel_id),
        embed,
        components,
    )
    .await
}

pub async fn handle_component(
    ctx: &Context,
    c: &ComponentInteraction,
    bot: &DiscordBot,
) -> AppResult<()> {
    match c.data.custom_id.as_str() {
        "hk:home" => {
            return c
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(hub_message().ephemeral(true)),
                )
                .await
                .map_err(|e| AppError::Discord(e.to_string()));
        }
        "hk:list" => return send_list(ctx, c, bot).await,
        "hk:list:refresh" => return refresh_lobby_list(ctx, c, bot).await,
        "hk:create" => return create_lobby(ctx, c, bot).await,
        "hk:whoishere" => return send_whoishere(ctx, c, bot, false).await,
        "hk:current" => return send_current_lobby(ctx, c, bot).await,
        "hk:steam" => return show_steam_modal(c, ctx).await,
        "hk:help" => return ephemeral(c, ctx, "Use Lobbies to open a lobby, Create lobby to start one, and the lobby panel buttons to join, leave, balance, or manage players.").await,
        _ => {}
    }
    if let Some((requester, action)) = parse_whoishere_id(&c.data.custom_id) {
        let balanced = match action {
            "balance" => true,
            "plain" | "refresh" => false,
            _ => return ephemeral(c, ctx, "This control is no longer available.").await,
        };
        return update_whoishere(ctx, c, bot, requester, balanced).await;
    }
    if let Some(value) = c.data.custom_id.strip_prefix("hk:open:") {
        let mut parts = value.split(':');
        let number = parts
            .next()
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| AppError::InvalidInput("invalid lobby control".into()))?;
        let id = parts
            .next()
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| AppError::InvalidInput("invalid lobby control".into()))?;
        let guild = c
            .guild_id
            .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
        let Some(_) =
            lobby_service::resolve_panel(&bot.pool, guild.get() as i64, number, id).await?
        else {
            let (embed, components) = ended(number);
            c.create_response(
                ctx,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .components(components),
                ),
            )
            .await
            .map_err(|e| AppError::Discord(e.to_string()))?;
            return Ok(());
        };
        let (embed, components) = panel(ctx, bot, guild.get(), number, id, false).await?;
        publish_component(
            ctx,
            c,
            bot,
            PanelKey::lobby(number, c.channel_id),
            embed,
            components,
        )
        .await?;
        return Ok(());
    }
    let Some((number, id, action)) = parse_panel_id(&c.data.custom_id) else {
        return ephemeral(c, ctx, "This control is no longer available.").await;
    };
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?
        .get() as i64;
    if lobby_service::resolve_panel(&bot.pool, guild, number, id)
        .await?
        .is_none()
    {
        c.create_response(ctx, CreateInteractionResponse::Acknowledge)
            .await
            .map_err(|e| AppError::Discord(e.to_string()))?;
        let (embed, components) = ended(number);
        c.create_response(
            ctx,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(components),
            ),
        )
        .await
        .map_err(|e| AppError::Discord(e.to_string()))?;
        return Ok(());
    }
    match action {
        "join" => {
            let display_name = member_name::interaction_name(&c.user, c.member.as_ref());
            let user = UserRepository::new(bot.pool.clone())
                .find_or_create(&c.user.id.to_string(), &display_name)
                .await?;
            if user.friend_code.is_none() {
                c.create_response(
                    ctx,
                    CreateInteractionResponse::Modal(lobby_steam_modal(number, id)),
                )
                .await
                .map_err(|e| AppError::Discord(e.to_string()))?;
                return Ok(());
            }
            lobby_service::join(&bot.pool, guild, number, &user, &bot.rank_service).await?;
            update_panel(c, ctx, bot, number, id, false).await
        }
        "leave" => {
            lobby_service::leave_lobby(&bot.pool, guild, number, &c.user.id.to_string()).await?;
            update_panel(c, ctx, bot, number, id, false).await
        }
        "balance" | "plain" => update_panel(c, ctx, bot, number, id, action == "balance").await,
        "refresh" => update_panel(c, ctx, bot, number, id, false).await,
        "manage" => show_manage(ctx, c, bot, number, id).await,
        "add" => {
            let Some(ComponentInteractionDataKind::UserSelect { values }) = Some(&c.data.kind)
            else {
                return ephemeral(c, ctx, "Choose a Discord member to add.").await;
            };
            let target_id = values
                .first()
                .ok_or_else(|| AppError::InvalidInput("choose a player to add".into()))?;
            let target = UserRepository::new(bot.pool.clone())
                .find_by_discord(&target_id.to_string())
                .await?
                .ok_or_else(|| {
                    AppError::InvalidInput("that player must link Steam before being added".into())
                })?;
            lobby_service::add(
                &bot.pool,
                guild,
                number,
                &target,
                &c.user.id.to_string(),
                &bot.rank_service,
            )
            .await?;
            bot.panel_registry
                .bump(&ctx.http, PanelKey::lobby(number, c.channel_id), || {
                    panel(ctx, bot, guild as u64, number, id, false)
                })
                .await?;
            ephemeral(c, ctx, "Player added to the lobby.").await
        }
        "remove" => {
            let Some(ComponentInteractionDataKind::StringSelect { values }) = Some(&c.data.kind)
            else {
                return ephemeral(c, ctx, "Choose a player to remove.").await;
            };
            let target = values
                .first()
                .ok_or_else(|| AppError::InvalidInput("choose a player to remove".into()))?;
            lobby_service::remove(&bot.pool, guild, number, target).await?;
            bot.panel_registry
                .bump(&ctx.http, PanelKey::lobby(number, c.channel_id), || {
                    panel(ctx, bot, guild as u64, number, id, false)
                })
                .await?;
            ephemeral(c, ctx, "Player removed from the lobby.").await
        }
        "close" => {
            lobby_service::close(&bot.pool, guild, number).await?;
            ephemeral(c, ctx, format!("Lobby #{number} closed.")).await?;
            bot.panel_registry
                .bump(&ctx.http, PanelKey::lobby(number, c.channel_id), || async {
                    Ok(ended(number))
                })
                .await?;
            Ok(())
        }
        _ => ephemeral(c, ctx, "This control is no longer available.").await,
    }
}

async fn send_list(ctx: &Context, c: &ComponentInteraction, bot: &DiscordBot) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    let lobbies = lobby_service::active(&bot.pool, guild.get() as i64).await?;
    let (embed, rows) = lobby_list_view(&lobbies);
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .embed(embed)
                .components(rows)
                .ephemeral(true),
        ),
    )
    .await
    .map_err(|e| AppError::Discord(e.to_string()))
}

async fn create_lobby(ctx: &Context, c: &ComponentInteraction, bot: &DiscordBot) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    let lobby =
        lobby_service::create(&bot.pool, guild.get() as i64, &c.user.id.to_string(), None).await?;
    let (embed, components) = panel(ctx, bot, guild.get(), lobby.number, lobby.id, false).await?;
    publish_component(
        ctx,
        c,
        bot,
        PanelKey::lobby(lobby.number, c.channel_id),
        embed,
        components,
    )
    .await
}

async fn show_manage(
    ctx: &Context,
    c: &ComponentInteraction,
    bot: &DiscordBot,
    number: i32,
    id: i64,
) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    crate::services::moderation::permission::require_role_manager_for(
        &bot.pool,
        bot,
        guild.get() as i64,
        &c.user,
        c.member.as_ref(),
    )
    .await?;
    let (_, players) = lobby_service::list(
        &bot.pool,
        guild.get() as i64,
        Some(number),
        &c.user.id.to_string(),
    )
    .await?;
    let mut options = Vec::with_capacity(players.len().min(25));
    for player in players.into_iter().take(25) {
        let display_name =
            member_name::guild_display_name(ctx, guild, &player.discord_id, &player.display_name)
                .await;
        options.push(CreateSelectMenuOption::new(display_name, player.discord_id));
    }
    let remove = CreateSelectMenu::new(
        panel_id(number, id, "remove"),
        CreateSelectMenuKind::String { options },
    )
    .placeholder("Remove a player");
    let add = CreateSelectMenu::new(
        panel_id(number, id, "add"),
        CreateSelectMenuKind::User {
            default_users: None,
        },
    )
    .placeholder("Add a player");
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(format!("Manage lobby #{number}"))
                .ephemeral(true)
                .components(vec![
                    CreateActionRow::SelectMenu(add),
                    CreateActionRow::SelectMenu(remove),
                    CreateActionRow::Buttons(vec![CreateButton::new(panel_id(
                        number, id, "close",
                    ))
                    .label("Close lobby")
                    .style(ButtonStyle::Danger)]),
                ]),
        ),
    )
    .await
    .map_err(|e| AppError::Discord(e.to_string()))?;
    Ok(())
}

async fn show_steam_modal(c: &ComponentInteraction, ctx: &Context) -> AppResult<()> {
    c.create_response(
        ctx,
        CreateInteractionResponse::Modal(
            CreateModal::new("hk:steam_modal", "Link Steam account").components(vec![
                CreateActionRow::InputText(
                    CreateInputText::new(InputTextStyle::Short, "Steam friend code", "friend_code")
                        .placeholder("digits, optionally with dashes"),
                ),
            ]),
        ),
    )
    .await
    .map_err(|e| AppError::Discord(e.to_string()))?;
    Ok(())
}

pub async fn handle_modal(ctx: &Context, c: &ModalInteraction, bot: &DiscordBot) -> AppResult<()> {
    let lobby_context = parse_lobby_steam_modal_id(&c.data.custom_id);
    if c.data.custom_id != "hk:steam_modal" && lobby_context.is_none() {
        return Err(AppError::InvalidInput(
            "This form is no longer available.".into(),
        ));
    }
    c.defer_ephemeral(ctx)
        .await
        .map_err(|error| AppError::Discord(error.to_string()))?;
    let result = async {
        if let Some((number, id)) = lobby_context {
            let guild = c
                .guild_id
                .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
            if lobby_service::resolve_panel(&bot.pool, guild.get() as i64, number, id)
                .await?
                .is_none()
            {
                return Err(AppError::InvalidInput("the lobby has ended".into()));
            }
        }
        let value = c
            .data
            .components
            .iter()
            .flat_map(|row| row.components.iter())
            .find_map(|component| match component {
                ActionRowComponent::InputText(input) if input.custom_id == "friend_code" => {
                    input.value.clone()
                }
                _ => None,
            })
            .ok_or_else(|| AppError::InvalidInput("friend code is required".into()))?;
        let display_name = member_name::interaction_name(&c.user, c.member.as_ref());
        register_service::link(
            &bot.pool,
            &bot.rank_service,
            &c.user.id.to_string(),
            &display_name,
            &value,
        )
        .await?;

        if let Some((number, id)) = lobby_context {
            let guild = c
                .guild_id
                .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
            let guild_id = guild.get() as i64;
            let lobby = lobby_service::resolve_panel(&bot.pool, guild_id, number, id)
                .await?
                .ok_or_else(|| AppError::InvalidInput("the lobby has ended".into()))?;
            let user = UserRepository::new(bot.pool.clone())
                .find_by_discord(&c.user.id.to_string())
                .await?
                .ok_or_else(|| {
                    AppError::InvalidInput("Steam account could not be loaded".into())
                })?;
            lobby_service::join_with_snapshot(&bot.pool, guild_id, &lobby, &user).await?;
            bot.panel_registry
                .bump(&ctx.http, PanelKey::lobby(number, c.channel_id), || {
                    panel(ctx, bot, guild.get(), number, id, false)
                })
                .await?;
            Ok(format!("Steam linked and you joined lobby #{number}."))
        } else {
            Ok("Steam account linked and Dota data refreshed.".into())
        }
    }
    .await;
    let content = match result {
        Ok(content) => content,
        Err(AppError::InvalidInput(message)) => message,
        Err(AppError::Forbidden) => "You are not allowed to perform that action.".into(),
        Err(error) => {
            tracing::error!(?error, "Steam-link modal failed");
            "Something went wrong. Please try again.".into()
        }
    };
    c.edit_response(ctx, EditInteractionResponse::new().content(content))
        .await
        .map_err(|error| AppError::Discord(error.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_lobby_steam_modal_id, parse_whoishere_id};

    #[test]
    fn parses_lobby_steam_modal_context() {
        assert_eq!(
            parse_lobby_steam_modal_id("hk:steam_lobby:7:42"),
            Some((7, 42))
        );
        assert_eq!(parse_lobby_steam_modal_id("hk:steam_lobby:0:42"), None);
        assert_eq!(parse_lobby_steam_modal_id("hk:steam_lobby:7:-1"), None);
        assert_eq!(
            parse_lobby_steam_modal_id("hk:steam_lobby:7:42:extra"),
            None
        );
        assert_eq!(parse_lobby_steam_modal_id("hk:steam_modal"), None);
    }

    #[test]
    fn parses_whoishere_controls_without_matching_lobby_controls() {
        assert_eq!(
            parse_whoishere_id("hk:w:123456789:balance"),
            Some(("123456789", "balance"))
        );
        assert_eq!(parse_whoishere_id("hk:l:7:42:refresh"), None);
        assert_eq!(
            parse_whoishere_id("hk:w:123456789:unknown"),
            Some(("123456789", "unknown"))
        );
    }
}
