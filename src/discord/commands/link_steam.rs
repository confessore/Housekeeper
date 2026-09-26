use crate::{
    database::UserRepository,
    discord::bot::DiscordBot,
    utils::error::{AppError, AppResult},
};
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};
pub fn register() -> CreateCommand {
    CreateCommand::new("link-steam")
        .description("Link your Steam friend code and refresh Dota data")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "friend_code",
                "8-digit Steam friend code",
            )
            .required(true),
        )
}
pub async fn run(ctx: &Context, c: &CommandInteraction, bot: &DiscordBot) -> AppResult<()> {
    let input = c
        .data
        .options
        .iter()
        .find(|o| o.name == "friend_code")
        .and_then(|o| o.value.as_str())
        .ok_or_else(|| AppError::InvalidInput("friend_code is required".into()))?;
    let friend_code = parse_friend_code(input)
        .ok_or_else(|| AppError::InvalidInput("provide your 8-digit Steam friend code".into()))?;
    let account_id = friend_code
        .parse::<i64>()
        .map_err(|_| AppError::InvalidInput("provide your 8-digit Steam friend code".into()))?;
    let user = UserRepository::new(bot.pool.clone())
        .find_or_create(&c.user.id.to_string(), &c.user.name)
        .await?;
    bot.rank_service
        .link(user.id, &friend_code, account_id)
        .await?;
    respond(
        ctx,
        c,
        "Steam account linked and Dota data refreshed.",
        true,
    )
    .await
}
pub fn parse_friend_code(input: &str) -> Option<String> {
    let code = input.trim().replace('-', "");
    (code.len() == 8 && code.chars().all(|character| character.is_ascii_digit()))
        .then_some(code)
        .filter(|code| code.parse::<i64>().is_ok_and(|account_id| account_id > 0))
}
async fn respond(
    ctx: &Context,
    c: &CommandInteraction,
    text: &str,
    ephemeral: bool,
) -> AppResult<()> {
    c.create_response(
        ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(text)
                .ephemeral(ephemeral),
        ),
    )
    .await
    .map_err(|e| AppError::Discord(e.to_string()))?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_friend_code() {
        assert_eq!(parse_friend_code("22945962"), Some("22945962".into()));
        assert_eq!(parse_friend_code("2294-5962"), Some("22945962".into()));
        assert_eq!(parse_friend_code("76561197960287930"), None);
        assert_eq!(parse_friend_code("nope"), None);
    }
}
