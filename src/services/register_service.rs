use crate::{
    database::UserRepository,
    services::rank_lookup_service::RankLookupService,
    utils::error::{AppError, AppResult},
};
use sqlx::PgPool;

pub fn parse_friend_code(input: &str) -> Option<String> {
    let code = input.trim().replace('-', "");
    (code.chars().all(|character| character.is_ascii_digit()))
        .then_some(code)
        .filter(|code| code.parse::<u32>().is_ok_and(|account_id| account_id > 0))
}

pub async fn link(
    pool: &PgPool,
    rank_service: &RankLookupService,
    discord_id: &str,
    display_name: &str,
    input: &str,
) -> AppResult<()> {
    let friend_code = parse_friend_code(input)
        .ok_or_else(|| AppError::InvalidInput("provide your Steam friend code".into()))?;
    let account_id = friend_code
        .parse::<i64>()
        .map_err(|_| AppError::InvalidInput("provide your Steam friend code".into()))?;
    let user = UserRepository::new(pool.clone())
        .find_or_create(discord_id, display_name)
        .await?;
    rank_service.link(user.id, &friend_code, account_id).await
}

#[cfg(test)]
mod tests {
    use super::parse_friend_code;

    #[test]
    fn parses_friend_code() {
        assert_eq!(parse_friend_code("22945962"), Some("22945962".into()));
        assert_eq!(parse_friend_code("2294-5962"), Some("22945962".into()));
        assert_eq!(parse_friend_code("123456789"), Some("123456789".into()));
        assert_eq!(parse_friend_code("4294967295"), Some("4294967295".into()));
        assert_eq!(parse_friend_code("4294967296"), None);
        assert_eq!(parse_friend_code("76561197960287930"), None);
        assert_eq!(parse_friend_code("nope"), None);
    }
}
