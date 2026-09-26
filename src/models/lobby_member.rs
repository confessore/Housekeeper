#[derive(Debug, Clone)]
pub struct LobbyMember {
    pub discord_id: String,
    pub rank_tier: Option<i32>,
    pub wins: i32,
    pub losses: i32,
}
