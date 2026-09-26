use crate::models::{Rank, RoleTier, User};
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceMember {
    pub discord_id: String,
    pub username: String,
    pub bot: bool,
    pub channel_id: Option<u64>,
}

pub fn members_in_channel(caller_id: &str, members: &[VoiceMember]) -> Vec<(String, String)> {
    let channel_id = members
        .iter()
        .find(|member| member.discord_id == caller_id)
        .and_then(|member| member.channel_id);
    let Some(channel_id) = channel_id else {
        return Vec::new();
    };
    members
        .iter()
        .filter(|member| !member.bot && member.channel_id == Some(channel_id))
        .map(|member| (member.discord_id.clone(), member.username.clone()))
        .collect()
}

#[derive(Debug, Clone)]
pub struct PlayerView {
    pub discord_id: String,
    pub display_name: String,
    pub rank: Rank,
    pub role: Option<RoleTier>,
    pub games: i32,
    pub wins: i32,
    pub losses: i32,
}
pub fn sort_players(players: &mut [PlayerView]) {
    players.sort_by(|a, b| {
        let rank = b.rank.order().cmp(&a.rank.order());
        if rank == Ordering::Equal {
            b.games
                .cmp(&a.games)
                .then_with(|| a.display_name.cmp(&b.display_name))
        } else {
            rank
        }
    })
}
pub fn view(user: User, role: Option<RoleTier>) -> PlayerView {
    view_with_stats(user, role, None, None, None)
}

pub fn view_with_stats(
    user: User,
    role: Option<RoleTier>,
    rank_tier: Option<i32>,
    wins: Option<i32>,
    losses: Option<i32>,
) -> PlayerView {
    let User {
        discord_id,
        username,
        rank_tier: current_rank_tier,
        wins: current_wins,
        losses: current_losses,
        ..
    } = user;
    let rank_tier = rank_tier.or(current_rank_tier);
    let wins = wins.unwrap_or(current_wins);
    let losses = losses.unwrap_or(current_losses);
    PlayerView {
        discord_id,
        display_name: username,
        rank: Rank::from_tier(rank_tier),
        role,
        games: wins + losses,
        wins,
        losses,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_a_large_voice_channel_without_discord() {
        let mut members = (0..12)
            .map(|index| VoiceMember {
                discord_id: index.to_string(),
                username: format!("player-{index}"),
                bot: false,
                channel_id: Some(7),
            })
            .collect::<Vec<_>>();
        members.push(VoiceMember {
            discord_id: "spectator".into(),
            username: "spectator".into(),
            bot: false,
            channel_id: Some(8),
        });
        members.push(VoiceMember {
            discord_id: "bot".into(),
            username: "bot".into(),
            bot: true,
            channel_id: Some(7),
        });

        let players = members_in_channel("0", &members);
        assert_eq!(players.len(), 12);
        assert!(!players.iter().any(|(id, _)| id == "bot"));
        assert!(!players.iter().any(|(id, _)| id == "spectator"));
    }
}
