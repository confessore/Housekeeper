use crate::{
    database::UserRepository, external::opendota_client::OpenDotaClient, models::User,
    utils::error::AppResult,
};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct RankLookupService {
    pool: PgPool,
    client: OpenDotaClient,
}

impl RankLookupService {
    pub fn new(pool: PgPool, key: Option<String>, requests_per_minute: u32) -> Arc<Self> {
        Arc::new(Self {
            pool: pool.clone(),
            client: OpenDotaClient::new(key, requests_per_minute),
        })
    }

    pub async fn link(
        &self,
        user_id: i64,
        friend_code: &str,
        account_id: i64,
    ) -> AppResult<Option<String>> {
        self.sync(user_id, friend_code, account_id).await
    }

    pub async fn refresh(&self, user: &User) -> AppResult<User> {
        let Some(friend_code) = user.friend_code.as_deref() else {
            return Ok(user.clone());
        };
        let Ok(account_id) = friend_code.parse::<i64>() else {
            return Ok(user.clone());
        };
        self.sync(user.id, friend_code, account_id).await?;
        Ok(UserRepository::new(self.pool.clone())
            .find_by_discord(&user.discord_id)
            .await?
            .unwrap_or_else(|| user.clone()))
    }

    async fn sync(
        &self,
        user_id: i64,
        friend_code: &str,
        account_id: i64,
    ) -> AppResult<Option<String>> {
        let (player, wl) = self.client.player_and_win_loss(account_id).await?;
        UserRepository::new(self.pool.clone())
            .update_friend_code(user_id, friend_code, player.rank_tier, wl.win, wl.lose)
            .await?;
        Ok(player
            .profile
            .and_then(|profile| profile.personaname)
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty()))
    }
}
