use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rank {
    Herald,
    Guardian,
    Crusader,
    Archon,
    Legend,
    Ancient,
    Divine,
    Immortal,
    Unranked,
}
impl Rank {
    pub fn from_tier(tier: Option<i32>) -> Self {
        match tier.unwrap_or_default() {
            10..=19 => Self::Herald,
            20..=29 => Self::Guardian,
            30..=39 => Self::Crusader,
            40..=49 => Self::Archon,
            50..=59 => Self::Legend,
            60..=69 => Self::Ancient,
            70..=79 => Self::Divine,
            80 => Self::Immortal,
            _ => Self::Unranked,
        }
    }
    pub fn order(self) -> i32 {
        match self {
            Self::Herald => 0,
            Self::Guardian => 1,
            Self::Crusader => 2,
            Self::Archon => 3,
            Self::Legend => 4,
            Self::Ancient => 5,
            Self::Divine => 6,
            Self::Immortal => 7,
            Self::Unranked => -1,
        }
    }
    pub fn badge(self) -> &'static str {
        match self {
            Self::Herald => "Herald",
            Self::Guardian => "Guardian",
            Self::Crusader => "Crusader",
            Self::Archon => "Archon",
            Self::Legend => "Legend",
            Self::Ancient => "Ancient",
            Self::Divine => "Divine",
            Self::Immortal => "Immortal",
            Self::Unranked => "Unranked",
        }
    }
}
impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.badge())
    }
}
impl Ord for Rank {
    fn cmp(&self, other: &Self) -> Ordering {
        self.order().cmp(&other.order())
    }
}
impl PartialOrd for Rank {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tiers_map() {
        assert_eq!(Rank::from_tier(Some(80)), Rank::Immortal);
        assert_eq!(Rank::from_tier(Some(10)), Rank::Herald);
        assert_eq!(Rank::from_tier(None), Rank::Unranked);
    }
}
