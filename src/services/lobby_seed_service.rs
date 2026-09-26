use crate::{
    database::{LobbyRepository, UserRepository},
    models::User,
    services::{lobby_service, rank_lookup_service::RankLookupService},
    utils::error::{AppError, AppResult},
};
use sqlx::PgPool;

const SYNTHETIC_PREFIX: &str = "housekeeper-test-";
const SYNTHETIC_PROFILES: [(Option<i32>, i32, i32); 10] = [
    (Some(11), 18, 22),
    (Some(21), 1_200, 1_400),
    (Some(31), 90, 90),
    (Some(41), 1_700, 1_500),
    (Some(51), 320, 240),
    (Some(61), 70, 70),
    (Some(71), 1_400, 800),
    (Some(80), 900, 600),
    (None, 400, 1_200),
    (None, 1_200, 200),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticPlayer {
    pub discord_id: String,
    pub username: String,
    pub rank_tier: Option<i32>,
    pub wins: i32,
    pub losses: i32,
}

pub fn synthetic_players(count: i64) -> AppResult<Vec<SyntheticPlayer>> {
    if !(1..=lobby_service::CAPACITY).contains(&count) {
        return Err(AppError::InvalidInput(format!(
            "count must be between 1 and {}",
            lobby_service::CAPACITY
        )));
    }
    Ok((0..count as usize)
        .map(|index| {
            let (rank_tier, wins, losses) = SYNTHETIC_PROFILES[index];
            SyntheticPlayer {
                discord_id: format!("{SYNTHETIC_PREFIX}{}", index + 1),
                username: format!("Test Player {}", index + 1),
                rank_tier,
                wins,
                losses,
            }
        })
        .collect())
}

pub async fn seed(
    pool: &PgPool,
    discord_guild_id: i64,
    lobby_number: i32,
    count: i64,
    rank_service: &RankLookupService,
) -> AppResult<Vec<User>> {
    let players = synthetic_players(count)?;
    let guild_id = lobby_service::resolve_guild_id(pool, discord_guild_id).await?;
    let lobby = lobby_service::resolve(pool, guild_id, lobby_number).await?;
    let repo = LobbyRepository::new(pool);
    let available = lobby_service::CAPACITY - repo.count(lobby.id).await?;
    if count > available {
        return Err(AppError::InvalidInput(format!(
            "the lobby only has room for {available} more players"
        )));
    }

    let users = UserRepository::new(pool.clone());
    let mut seeded = Vec::with_capacity(players.len());
    for player in players {
        let user = users
            .upsert_synthetic(
                &player.discord_id,
                &player.username,
                player.rank_tier,
                player.wins,
                player.losses,
            )
            .await?;
        lobby_service::add(
            pool,
            discord_guild_id,
            lobby_number,
            &user,
            &format!("{SYNTHETIC_PREFIX}seed"),
            rank_service,
        )
        .await?;
        seeded.push(user);
    }
    Ok(seeded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Rank;

    #[test]
    fn creates_a_full_rank_spread() {
        let players = synthetic_players(10).unwrap();
        assert_eq!(players.len(), 10);
        assert_eq!(
            players
                .iter()
                .map(|player| Rank::from_tier(player.rank_tier))
                .collect::<Vec<_>>(),
            vec![
                Rank::Herald,
                Rank::Guardian,
                Rank::Crusader,
                Rank::Archon,
                Rank::Legend,
                Rank::Ancient,
                Rank::Divine,
                Rank::Immortal,
                Rank::Unranked,
                Rank::Unranked,
            ]
        );
    }

    #[test]
    fn profiles_have_varied_ranks_and_realistic_game_counts() {
        let players = synthetic_players(10).unwrap();
        let ranked_tiers = players
            .iter()
            .filter_map(|player| player.rank_tier)
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(ranked_tiers.len(), 8);
        assert_eq!(players[0].wins + players[0].losses, 40);
        assert_eq!(players[1].rank_tier, Some(21));
        assert_eq!(players[1].wins + players[1].losses, 2_600);
        assert!(players[1].wins < players[1].losses);
        assert_eq!(players[2].wins, players[2].losses);
        assert!(players[3].wins > players[3].losses);
        assert_eq!(players[7].wins + players[7].losses, 1_500);
        assert_eq!(players[8].rank_tier, None);
        assert_eq!(players[8].wins + players[8].losses, 1_600);
        assert_eq!(players[9].rank_tier, None);
        assert_eq!(players[9].wins + players[9].losses, 1_400);
        assert!(players
            .iter()
            .all(|player| player.wins + player.losses >= 10));
    }

    #[test]
    fn rejects_invalid_counts() {
        assert!(synthetic_players(0).is_err());
        assert!(synthetic_players(11).is_err());
    }
}
