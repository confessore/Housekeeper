use crate::{
    database::UserRepository,
    discord::bot::DiscordBot,
    services::{balance_service, lobby_service, register_service},
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    ActionRowComponent, ButtonStyle, CommandInteraction, ComponentInteraction,
    ComponentInteractionDataKind, Context, CreateActionRow, CreateButton, CreateEmbed,
    CreateInputText, CreateInteractionResponse, CreateInteractionResponseMessage, CreateModal,
    CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption, InputTextStyle,
    ModalInteraction,
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

fn lobby_buttons(number: i32, id: i64, balanced: bool) -> Vec<CreateActionRow> {
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
    let (lobby, players) =
        lobby_service::list(&bot.pool, guild_id as i64, Some(number), "").await?;
    let mut embed =
        CreateEmbed::new().title(format!("Lobby #{} ({}/10)", lobby.number, players.len()));
    let roster = if players.is_empty() {
        "No players yet.".to_string()
    } else {
        players
            .iter()
            .enumerate()
            .map(|(index, player)| format!("{}. <@{}>", index + 1, player.discord_id))
            .collect::<Vec<_>>()
            .join("\n")
    };
    embed = embed.description(format!(
        "Expires in {} minutes\n\n{}",
        lobby.minutes_remaining(),
        roster
    ));
    if balanced {
        let teams = balance_service::split(&players);
        embed = embed
            .field(
                "Radiant",
                teams
                    .first
                    .iter()
                    .map(|p| format!("<@{}>", p.discord_id))
                    .collect::<Vec<_>>()
                    .join("\n"),
                true,
            )
            .field(
                "Dire",
                teams
                    .second
                    .iter()
                    .map(|p| format!("<@{}>", p.discord_id))
                    .collect::<Vec<_>>()
                    .join("\n"),
                true,
            );
    }
    let _ = ctx;
    Ok((embed, lobby_buttons(number, id, balanced)))
}

fn ended(number: i32) -> (CreateEmbed, Vec<CreateActionRow>) {
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
                .description("Use the buttons below to manage your lobbies without commands."),
        )
        .components(vec![CreateActionRow::Buttons(vec![
            CreateButton::new("hk:list")
                .label("Lobbies")
                .style(ButtonStyle::Primary),
            CreateButton::new("hk:create")
                .label("Create lobby")
                .style(ButtonStyle::Success),
            CreateButton::new("hk:steam")
                .label("Link Steam")
                .style(ButtonStyle::Secondary),
            CreateButton::new("hk:help")
                .label("Help")
                .style(ButtonStyle::Secondary),
        ])])
}

pub async fn send_hub(ctx: &Context, c: &CommandInteraction) -> AppResult<()> {
    c.create_response(ctx, CreateInteractionResponse::Message(hub_message()))
        .await
        .map_err(|error| AppError::Discord(error.to_string()))?;
    Ok(())
}

pub async fn send_lobby_panel(
    ctx: &Context,
    c: &CommandInteraction,
    bot: &DiscordBot,
    number: i32,
    balanced: bool,
) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    let lobby = lobby_service::resolve(&bot.pool, guild.get() as i64, number).await?;
    let (embed, components) =
        panel(ctx, bot, guild.get(), lobby.number, lobby.id, balanced).await?;
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .embed(embed)
                .components(components),
        ),
    )
    .await
    .map_err(|error| AppError::Discord(error.to_string()))?;
    Ok(())
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
    let mut embed = CreateEmbed::new().title("Active lobbies");
    let mut rows = Vec::new();
    for (lobby, count) in lobbies.iter().take(20) {
        embed = embed.field(
            format!("Lobby #{}", lobby.number),
            format!(
                "{count}/10 players — {} minutes remaining",
                lobby.minutes_remaining()
            ),
            false,
        );
        rows.push(
            CreateButton::new(format!("hk:open:{}:{}", lobby.number, lobby.id))
                .label(format!("Open #{}", lobby.number))
                .style(ButtonStyle::Primary),
        );
    }
    rows.push(
        CreateButton::new("hk:create")
            .label("Create lobby")
            .style(ButtonStyle::Success),
    );
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .embed(embed.description(if lobbies.is_empty() {
                    "No active lobbies."
                } else {
                    "Select a lobby to open its control panel."
                }))
                .components(vec![CreateActionRow::Buttons(
                    rows.into_iter().take(5).collect(),
                )]),
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

async fn update_panel(
    c: &ComponentInteraction,
    ctx: &Context,
    bot: &DiscordBot,
    number: i32,
    id: i64,
    balanced: bool,
) -> AppResult<()> {
    let Some(_) = lobby_service::resolve_panel(
        &bot.pool,
        c.guild_id
            .ok_or_else(|| AppError::InvalidInput("server only".into()))?
            .get() as i64,
        number,
        id,
    )
    .await?
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
    let (embed, components) =
        panel(ctx, bot, c.guild_id.unwrap().get(), number, id, balanced).await?;
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
    Ok(())
}

pub async fn handle_component(
    ctx: &Context,
    c: &ComponentInteraction,
    bot: &DiscordBot,
) -> AppResult<()> {
    match c.data.custom_id.as_str() {
        "hk:list" => return send_list(ctx, c, bot).await,
        "hk:create" => return create_lobby(ctx, c, bot).await,
        "hk:steam" => return show_steam_modal(c, ctx).await,
        "hk:help" => return ephemeral(c, ctx, "Use Lobbies to open a lobby, Create lobby to start one, and the lobby panel buttons to join, leave, balance, or manage players.").await,
        _ => {}
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
            let user = UserRepository::new(bot.pool.clone())
                .find_or_create(&c.user.id.to_string(), &c.user.name)
                .await?;
            if user.friend_code.is_none() {
                c.create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("Link Steam before joining this lobby.")
                            .ephemeral(true)
                            .components(vec![CreateActionRow::Buttons(vec![CreateButton::new(
                                "hk:steam",
                            )
                            .label("Link Steam")
                            .style(ButtonStyle::Primary)])]),
                    ),
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
            ephemeral(c, ctx, "Player removed from the lobby.").await
        }
        "close" => {
            lobby_service::close(&bot.pool, guild, number).await?;
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
    buttons.push(
        CreateButton::new("hk:create")
            .label("Create lobby")
            .style(ButtonStyle::Success),
    );
    c.create_response(
        ctx,
        CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::new()
                .embed(embed.description(if lobbies.is_empty() {
                    "No active lobbies."
                } else {
                    "Select a lobby to open its control panel."
                }))
                .components(vec![CreateActionRow::Buttons(
                    buttons.into_iter().take(5).collect(),
                )]),
        ),
    )
    .await
    .map_err(|e| AppError::Discord(e.to_string()))?;
    Ok(())
}

async fn create_lobby(ctx: &Context, c: &ComponentInteraction, bot: &DiscordBot) -> AppResult<()> {
    let guild = c
        .guild_id
        .ok_or_else(|| AppError::InvalidInput("server only".into()))?;
    let lobby =
        lobby_service::create(&bot.pool, guild.get() as i64, &c.user.id.to_string(), None).await?;
    let (embed, components) = panel(ctx, bot, guild.get(), lobby.number, lobby.id, false).await?;
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
    Ok(())
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
    let options = players
        .into_iter()
        .take(25)
        .map(|p| CreateSelectMenuOption::new(p.display_name, p.discord_id))
        .collect();
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
                        .placeholder("12345678 or 1234-5678"),
                ),
            ]),
        ),
    )
    .await
    .map_err(|e| AppError::Discord(e.to_string()))?;
    Ok(())
}

pub async fn handle_modal(ctx: &Context, c: &ModalInteraction, bot: &DiscordBot) -> AppResult<()> {
    if c.data.custom_id != "hk:steam_modal" {
        return Err(AppError::InvalidInput(
            "This form is no longer available.".into(),
        ));
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
    register_service::link(
        &bot.pool,
        &bot.rank_service,
        &c.user.id.to_string(),
        &c.user.name,
        &value,
    )
    .await?;
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("Steam account linked and Dota data refreshed.")
                .ephemeral(true),
        ),
    )
    .await
    .map_err(|e| AppError::Discord(e.to_string()))?;
    Ok(())
}
