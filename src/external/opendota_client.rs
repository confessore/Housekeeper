use crate::utils::error::{AppError, AppResult};
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
    time::Duration,
};
use tokio::{sync::Mutex, time::Instant};

const BASE: &str = "https://api.opendota.com/api";
const MAX_RETRIES: u32 = 3;
const LOOKUP_CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const MAX_RETRY_AFTER_WAIT: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct OpenDotaClient {
    client: reqwest::Client,
    key: Option<String>,
    next_request: Arc<Mutex<Instant>>,
    request_interval: Duration,
    lookup_locks: Arc<Mutex<HashMap<i64, Weak<Mutex<()>>>>>,
    lookup_cache: Arc<Mutex<HashMap<i64, CachedLookup>>>,
}

#[derive(Clone)]
struct CachedLookup {
    fetched_at: Instant,
    player: Player,
    win_loss: WinLoss,
}

impl CachedLookup {
    fn is_fresh(&self, now: Instant) -> bool {
        now.duration_since(self.fetched_at) < LOOKUP_CACHE_TTL
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Player {
    pub rank_tier: Option<i32>,
    pub profile: Option<PlayerProfile>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PlayerProfile {
    pub personaname: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WinLoss {
    pub win: i32,
    pub lose: i32,
}

impl OpenDotaClient {
    pub fn new(key: Option<String>, requests_per_minute: u32) -> Self {
        let request_interval = Self::request_interval(requests_per_minute);
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            key,
            next_request: Arc::new(Mutex::new(Instant::now())),
            request_interval,
            lookup_locks: Arc::new(Mutex::new(HashMap::new())),
            lookup_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn request_interval(requests_per_minute: u32) -> Duration {
        Duration::from_secs_f64(60.0 / requests_per_minute.max(1) as f64)
    }

    async fn throttle(&self) {
        let mut next_request = self.next_request.lock().await;
        let now = tokio::time::Instant::now();
        let wait = (*next_request).saturating_duration_since(now);
        *next_request = now.max(*next_request) + self.request_interval;
        drop(next_request);
        tokio::time::sleep(wait).await;
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> AppResult<T> {
        for attempt in 0..=MAX_RETRIES {
            self.throttle().await;
            let mut req = self.client.get(format!("{BASE}{path}"));
            if let Some(k) = &self.key {
                req = req.query(&[("api_key", k)]);
            }
            let response = match req.send().await {
                Ok(response) => response,
                Err(_error) if attempt < MAX_RETRIES => {
                    tokio::time::sleep(Self::retry_delay(attempt)).await;
                    continue;
                }
                Err(error) => return Err(AppError::External(error.to_string())),
            };

            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                if attempt < MAX_RETRIES {
                    let delay =
                        Self::retry_after(&response).unwrap_or_else(|| Self::retry_delay(attempt));
                    if !Self::retry_after_allowed(delay) {
                        return Err(AppError::External(
                            "OpenDota rate limited; Retry-After exceeds the retry window".into(),
                        ));
                    }
                    tokio::time::sleep(delay).await;
                    continue;
                }
                return Err(AppError::External(
                    "OpenDota rate limited; try again later".into(),
                ));
            }
            if response.status().is_server_error() && attempt < MAX_RETRIES {
                tokio::time::sleep(Self::retry_delay(attempt)).await;
                continue;
            }
            if !response.status().is_success() {
                return Err(AppError::External(format!(
                    "OpenDota returned {}",
                    response.status()
                )));
            }
            return response
                .json()
                .await
                .map_err(|e| AppError::External(e.to_string()));
        }
        unreachable!("the OpenDota retry loop always returns")
    }

    fn retry_delay(attempt: u32) -> Duration {
        Duration::from_secs(1 << attempt)
    }

    fn retry_after(response: &reqwest::Response) -> Option<Duration> {
        response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(Self::parse_retry_after)
    }

    fn parse_retry_after(value: &str) -> Option<Duration> {
        if let Ok(seconds) = value.parse::<u64>() {
            return Some(Duration::from_secs(seconds));
        }
        let retry_at = chrono::DateTime::parse_from_rfc2822(value)
            .ok()?
            .with_timezone(&chrono::Utc);
        Some(
            retry_at
                .signed_duration_since(chrono::Utc::now())
                .to_std()
                .unwrap_or(Duration::ZERO),
        )
    }

    fn retry_after_allowed(delay: Duration) -> bool {
        delay <= MAX_RETRY_AFTER_WAIT
    }

    async fn lookup_lock(&self, id: i64) -> Arc<Mutex<()>> {
        let mut locks = self.lookup_locks.lock().await;
        locks.retain(|_, lock| lock.strong_count() > 0);
        if let Some(lock) = locks.get(&id).and_then(Weak::upgrade) {
            return lock;
        }
        let lock = Arc::new(Mutex::new(()));
        locks.insert(id, Arc::downgrade(&lock));
        lock
    }

    pub async fn player_and_win_loss(&self, id: i64) -> AppResult<(Player, WinLoss)> {
        let account_lock = self.lookup_lock(id).await;
        let _guard = account_lock.lock().await;
        let now = Instant::now();
        {
            let mut cache = self.lookup_cache.lock().await;
            cache.retain(|_, lookup| lookup.is_fresh(now));
            if let Some(lookup) = cache.get(&id) {
                return Ok((lookup.player.clone(), lookup.win_loss.clone()));
            }
        }
        let player: Player = self.get(&format!("/players/{id}")).await?;
        let win_loss: WinLoss = self.get(&format!("/players/{id}/wl")).await?;
        self.lookup_cache.lock().await.insert(
            id,
            CachedLookup {
                fetched_at: Instant::now(),
                player: player.clone(),
                win_loss: win_loss.clone(),
            },
        );
        Ok((player, win_loss))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_optional_steam_persona_name() {
        let player: Player =
            serde_json::from_str(r#"{"rank_tier":42,"profile":{"personaname":"Steam Player"}}"#)
                .unwrap();
        assert_eq!(player.rank_tier, Some(42));
        assert_eq!(
            player.profile.and_then(|profile| profile.personaname),
            Some("Steam Player".into())
        );

        let player: Player = serde_json::from_str(r#"{"rank_tier":null}"#).unwrap();
        assert!(player.profile.is_none());
    }

    #[test]
    fn request_interval_respects_minimum_rate() {
        assert_eq!(OpenDotaClient::request_interval(60), Duration::from_secs(1));
        assert_eq!(OpenDotaClient::request_interval(0), Duration::from_secs(60));
    }

    #[test]
    fn cache_expires_after_five_minutes() {
        let fetched_at = Instant::now();
        let lookup = CachedLookup {
            fetched_at,
            player: Player {
                rank_tier: None,
                profile: None,
            },
            win_loss: WinLoss { win: 0, lose: 0 },
        };
        assert!(lookup.is_fresh(fetched_at + LOOKUP_CACHE_TTL - Duration::from_secs(1)));
        assert!(!lookup.is_fresh(fetched_at + LOOKUP_CACHE_TTL));
    }

    #[test]
    fn retry_after_is_not_shortened_or_retried_beyond_the_wait_limit() {
        let delay = OpenDotaClient::parse_retry_after("60").unwrap();
        assert_eq!(delay, Duration::from_secs(60));
        assert!(!OpenDotaClient::retry_after_allowed(delay));
        assert!(OpenDotaClient::retry_after_allowed(Duration::from_secs(30)));
        assert_eq!(
            OpenDotaClient::parse_retry_after("Tue, 15 Nov 1994 08:12:31 GMT"),
            Some(Duration::ZERO)
        );
    }

    #[tokio::test]
    async fn account_locks_are_shared_only_for_the_same_account() {
        let client = OpenDotaClient::new(None, 60);
        let first = client.lookup_lock(42).await;
        let same = client.lookup_lock(42).await;
        let other = client.lookup_lock(43).await;
        assert!(Arc::ptr_eq(&first, &same));
        assert!(!Arc::ptr_eq(&first, &other));
    }
}
