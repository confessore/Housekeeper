use crate::utils::error::{AppError, AppResult};
use serde::Deserialize;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

const BASE: &str = "https://api.opendota.com/api";
const MAX_RETRIES: u32 = 3;

#[derive(Clone)]
pub struct OpenDotaClient {
    client: reqwest::Client,
    key: Option<String>,
    next_request: Arc<Mutex<tokio::time::Instant>>,
    request_interval: Duration,
}

#[derive(Debug, Deserialize)]
pub struct Player {
    pub rank_tier: Option<i32>,
}

#[derive(Debug, Deserialize)]
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
            next_request: Arc::new(Mutex::new(tokio::time::Instant::now())),
            request_interval,
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
            .and_then(|value| value.parse::<u64>().ok())
            .map(|seconds| Duration::from_secs(seconds.min(30)))
    }

    pub async fn player(&self, id: i64) -> AppResult<Player> {
        self.get(&format!("/players/{id}")).await
    }

    pub async fn win_loss(&self, id: i64) -> AppResult<WinLoss> {
        self.get(&format!("/players/{id}/wl")).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_interval_respects_minimum_rate() {
        assert_eq!(OpenDotaClient::request_interval(60), Duration::from_secs(1));
        assert_eq!(OpenDotaClient::request_interval(0), Duration::from_secs(60));
    }
}
