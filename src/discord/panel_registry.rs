use crate::utils::error::{AppError, AppResult};
use serenity::all::{
    CacheHttp, ChannelId, CreateActionRow, CreateEmbed, CreateMessage, EditMessage, Message,
    MessageId,
};
use std::{
    collections::HashMap,
    future::Future,
    sync::{Arc, Mutex},
};
use tokio::sync::Mutex as AsyncMutex;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PanelKey {
    pub scope: String,
    pub key: String,
    pub channel_id: ChannelId,
}

impl PanelKey {
    pub fn lobby(number: i32, channel_id: ChannelId) -> Self {
        Self {
            scope: "lobby".into(),
            key: number.to_string(),
            channel_id,
        }
    }
}

type MessageSlot = Arc<AsyncMutex<Option<MessageId>>>;

#[derive(Default)]
pub struct PanelRegistry {
    // ponytail: this map grows with each channel/entity pair; pruning requires durable message tracking.
    messages: Mutex<HashMap<PanelKey, MessageSlot>>,
}

impl PanelRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn slot(&self, key: &PanelKey) -> MessageSlot {
        let mut messages = self.messages.lock().expect("panel registry lock poisoned");
        messages
            .entry(key.clone())
            .or_insert_with(|| Arc::new(AsyncMutex::new(None)))
            .clone()
    }

    pub async fn remember(&self, key: PanelKey, message_id: MessageId) {
        *self.slot(&key).lock().await = Some(message_id);
    }

    pub async fn evict(&self, http: impl CacheHttp, key: &PanelKey) {
        let slot = self.slot(key);
        let mut message = slot.lock().await;
        if let Some(message_id) = message.take() {
            let _ = key.channel_id.delete_message(http.http(), message_id).await;
        }
    }

    pub async fn replace<F, Fut>(
        &self,
        http: impl CacheHttp,
        key: PanelKey,
        send: F,
    ) -> AppResult<()>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = AppResult<MessageId>>,
    {
        let slot = self.slot(&key);
        let mut current = slot.lock().await;
        if let Some(message_id) = current.take() {
            let _ = key.channel_id.delete_message(http.http(), message_id).await;
        }
        *current = Some(send().await?);
        Ok(())
    }

    pub async fn bump<F, Fut>(
        &self,
        http: impl CacheHttp,
        key: PanelKey,
        render: F,
    ) -> AppResult<Message>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = AppResult<(CreateEmbed, Vec<CreateActionRow>)>>,
    {
        let slot = self.slot(&key);
        let mut current = slot.lock().await;
        if let Some(message_id) = current.take() {
            let _ = key.channel_id.delete_message(http.http(), message_id).await;
        }
        let (embed, components) = render().await?;
        let message = key
            .channel_id
            .send_message(
                http.http(),
                CreateMessage::new().embed(embed).components(components),
            )
            .await
            .map_err(|e| AppError::Discord(e.to_string()))?;
        *current = Some(message.id);
        Ok(message)
    }

    pub async fn publish<F, Fut>(
        &self,
        http: impl CacheHttp,
        key: PanelKey,
        render: F,
    ) -> AppResult<Message>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = AppResult<(CreateEmbed, Vec<CreateActionRow>)>>,
    {
        let slot = self.slot(&key);
        let mut current = slot.lock().await;
        let (embed, components) = render().await?;
        if let Some(message_id) = *current {
            if let Ok(message) = key
                .channel_id
                .edit_message(
                    http.http(),
                    message_id,
                    EditMessage::new()
                        .embed(embed.clone())
                        .components(components.clone()),
                )
                .await
            {
                return Ok(message);
            }
        }
        let message = key
            .channel_id
            .send_message(
                http.http(),
                CreateMessage::new().embed(embed).components(components),
            )
            .await
            .map_err(|e| AppError::Discord(e.to_string()))?;
        *current = Some(message.id);
        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_keys_are_scoped_by_channel_and_lobby_number() {
        let first = PanelKey::lobby(1, ChannelId::new(1));
        assert_eq!(first, PanelKey::lobby(1, ChannelId::new(1)));
        assert_ne!(first, PanelKey::lobby(2, ChannelId::new(1)));
        assert_ne!(first, PanelKey::lobby(1, ChannelId::new(2)));
    }

    #[tokio::test]
    async fn slots_are_shared_by_key() {
        let registry = PanelRegistry::new();
        let key = PanelKey::lobby(1, ChannelId::new(1));
        let first = registry.slot(&key);
        let _guard = first.lock().await;
        assert!(registry.slot(&key).try_lock().is_err());
        assert!(registry
            .slot(&PanelKey::lobby(2, ChannelId::new(1)))
            .try_lock()
            .is_ok());
    }
}
