#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleTier {
    pub id: i64,
    pub guild_id: i64,
    pub name: String,
    pub is_moderator: bool,
}
