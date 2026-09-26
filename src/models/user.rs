#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub discord_id: String,
    pub username: String,
    pub friend_code: Option<String>,
    pub rank_tier: Option<i32>,
    pub wins: i32,
    pub losses: i32,
    pub inhouse_banned: bool,
}
