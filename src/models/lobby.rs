use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Lobby {
    pub id: i64,
    pub number: i32,
    pub expires_at: DateTime<Utc>,
}

impl Lobby {
    pub fn minutes_remaining(&self) -> i64 {
        (self.expires_at - Utc::now()).num_minutes().max(0)
    }
}
