use crate::{
    database::{LobbyRepository, UserRepository},
    models::User,
    services::{lobby_service, rank_lookup_service::RankLookupService, register_service},
    utils::error::{AppError, AppResult},
};
use sqlx::PgPool;

pub(crate) const SYNTHETIC_PREFIX: &str = "housekeeper-test-";

const STEAM_FRIEND_CODES: [&str; 10] = [
    "100186894",
    "38633968",
    "92391843",
    "90984914",
    "154390881",
    "361263333",
    "85967961",
    "84868032",
    "39146344",
    "1020037084",
];

pub(crate) fn dummy_friend_code(discord_id: &str) -> String {
    discord_id
        .strip_prefix(SYNTHETIC_PREFIX)
        .map(|suffix| format!("{suffix:0>8}"))
        .unwrap_or_else(|| "00000000".into())
}

fn test_discord_id(friend_code: &str) -> String {
    format!("{SYNTHETIC_PREFIX}steam-{friend_code}")
}

fn validate_count(count: i64) -> AppResult<()> {
    if !(1..=lobby_service::CAPACITY).contains(&count) {
        return Err(AppError::InvalidInput(format!(
            "count must be between 1 and {}",
            lobby_service::CAPACITY
        )));
    }
    Ok(())
}

#[derive(Debug)]
pub struct SeedResult {
    pub added: Vec<User>,
    pub skipped: Vec<String>,
}

pub async fn seed(
    pool: &PgPool,
    discord_guild_id: i64,
    lobby_number: i32,
    count: i64,
    actor: &str,
    rank_service: &RankLookupService,
) -> AppResult<SeedResult> {
    validate_count(count)?;
    let guild_id = lobby_service::resolve_guild_id(pool, discord_guild_id).await?;
    let lobby = lobby_service::resolve(pool, guild_id, lobby_number).await?;
    let repo = LobbyRepository::new(pool);
    let available = lobby_service::CAPACITY - repo.count(lobby.id).await?;
    let users = UserRepository::new(pool.clone());
    let mut result = SeedResult {
        added: Vec::with_capacity(count.min(available).max(0) as usize),
        skipped: Vec::new(),
    };

    for friend_code in STEAM_FRIEND_CODES {
        if result.added.len() as i64 >= count || result.added.len() as i64 >= available {
            break;
        }
        let discord_id = test_discord_id(friend_code);
        if let Some(current) = repo.find_current_for_member(guild_id, &discord_id).await? {
            result.skipped.push(format!(
                "{friend_code} (already in lobby #{})",
                current.number
            ));
            continue;
        }

        let persona_name =
            match register_service::link(pool, rank_service, &discord_id, friend_code, friend_code)
                .await
            {
                Ok(name) => name,
                Err(AppError::External(_)) => {
                    result
                        .skipped
                        .push(format!("{friend_code} (could not load Steam data)"));
                    continue;
                }
                Err(error) => return Err(error),
            };
        let username = persona_name.as_deref().unwrap_or(friend_code);
        let user = users.find_or_create(&discord_id, username).await?;
        match lobby_service::add(
            pool,
            discord_guild_id,
            lobby_number,
            &user,
            actor,
            rank_service,
        )
        .await
        {
            Ok(()) => result
                .added
                .push(users.find_by_discord(&discord_id).await?.unwrap_or(user)),
            Err(AppError::InvalidInput(message)) => {
                result.skipped.push(format!("{friend_code} ({message})"));
                if message.contains("full")
                    || message.contains("filled up")
                    || message.contains("ended")
                {
                    break;
                }
            }
            Err(AppError::Forbidden) => {
                result
                    .skipped
                    .push(format!("{friend_code} (inhouse-banned)"));
            }
            Err(error) => return Err(error),
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::register_service::parse_friend_code;
    use std::collections::HashSet;

    #[test]
    fn steam_codes_are_valid_and_have_distinct_test_ids() {
        let friend_codes = STEAM_FRIEND_CODES
            .iter()
            .map(|code| parse_friend_code(code).expect("valid Steam friend code"))
            .collect::<Vec<_>>();
        let test_ids = friend_codes
            .iter()
            .map(|code| test_discord_id(code))
            .collect::<HashSet<_>>();
        assert_eq!(friend_codes.len(), 10);
        assert_eq!(test_ids.len(), friend_codes.len());
        assert_eq!(friend_codes[6], "85967961");
        assert_eq!(friend_codes[7], "84868032");
    }

    #[test]
    fn builds_distinct_dummy_friend_codes_for_existing_synthetic_players() {
        assert_eq!(dummy_friend_code("housekeeper-test-1"), "00000001");
        assert_eq!(dummy_friend_code("housekeeper-test-10"), "00000010");
        assert_eq!(dummy_friend_code("discord-user"), "00000000");
    }

    #[test]
    fn rejects_invalid_counts() {
        assert!(validate_count(0).is_err());
        assert!(validate_count(11).is_err());
        assert!(validate_count(1).is_ok());
        assert!(validate_count(10).is_ok());
    }
}
