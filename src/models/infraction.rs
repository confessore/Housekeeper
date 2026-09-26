use chrono::{DateTime, Utc};
#[derive(Debug, Clone, Copy)]
pub enum InfractionKind {
    Warn,
    Ban,
    Unban,
}
impl InfractionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Warn => "warn",
            Self::Ban => "ban",
            Self::Unban => "unban",
        }
    }
}
#[derive(Debug, Clone)]
pub struct Infraction {
    pub id: i64,
    pub target_discord_id: String,
    pub kind: String,
    pub reason: String,
    pub issued_by: String,
    pub created_at: DateTime<Utc>,
}
