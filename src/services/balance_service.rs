use crate::services::whoishere_service::PlayerView;
#[derive(Debug, Clone)]
pub struct TeamSplit {
    pub first: Vec<PlayerView>,
    pub second: Vec<PlayerView>,
}
fn balance_score(player: &PlayerView) -> i32 {
    if player.rank != crate::models::Rank::Unranked {
        return player.rank.order() * 100;
    }

    let games = player.games.max(0) as f64;
    let wins = player.wins.max(0) as f64;
    let losses = player.losses.max(0) as f64;
    let smoothed_win_rate = (wins + 10.0) / (games + losses + 20.0);
    ((smoothed_win_rate * 8.0 - 0.5) * 100.0)
        .round()
        .clamp(0.0, 700.0) as i32
}

pub fn split(players: &[PlayerView]) -> TeamSplit {
    let mut sorted = players.to_vec();
    sorted.sort_by(|a, b| {
        balance_score(b)
            .cmp(&balance_score(a))
            .then_with(|| b.games.cmp(&a.games))
    });
    let mut first = Vec::new();
    let mut second = Vec::new();
    for (i, p) in sorted.into_iter().enumerate() {
        if i % 2 == 0 {
            first.push(p)
        } else {
            second.push(p)
        }
    }
    TeamSplit { first, second }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Rank;
    #[test]
    fn splits() {
        let p = (0..4)
            .map(|i| PlayerView {
                discord_id: i.to_string(),
                display_name: i.to_string(),
                account_id: None,
                rank: Rank::Immortal,
                role: None,
                games: 1,
                wins: 1,
                losses: 0,
            })
            .collect::<Vec<_>>();
        let t = split(&p);
        assert_eq!(t.first.len(), 2);
        assert_eq!(t.second.len(), 2)
    }

    #[test]
    fn estimates_unranked_players_from_record_and_sample_size() {
        let low_sample = PlayerView {
            discord_id: "low".into(),
            display_name: "low".into(),
            account_id: None,
            rank: Rank::Unranked,
            role: None,
            games: 10,
            wins: 8,
            losses: 2,
        };
        let experienced = PlayerView {
            discord_id: "experienced".into(),
            display_name: "experienced".into(),
            account_id: None,
            rank: Rank::Unranked,
            role: None,
            games: 2_600,
            wins: 1_200,
            losses: 1_400,
        };
        let ranked_herald = PlayerView {
            discord_id: "herald".into(),
            display_name: "herald".into(),
            account_id: None,
            rank: Rank::Herald,
            role: None,
            games: 100,
            wins: 50,
            losses: 50,
        };

        assert!(balance_score(&low_sample) > balance_score(&ranked_herald));
        assert!(balance_score(&experienced) > balance_score(&ranked_herald));
        assert!(
            balance_score(&low_sample)
                < balance_score(&PlayerView {
                    rank: Rank::Immortal,
                    ..low_sample.clone()
                })
        );
    }
}
