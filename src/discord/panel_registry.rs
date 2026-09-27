use serenity::all::{
    CacheHttp, ChannelId, CreateActionRow, CreateEmbed, CreateMessage, EditMessage, Message,
};
use std::{collections::HashMap, sync::Mutex};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PanelKey {
    pub scope: String,
    pub key: String,
    pub channel_id: ChannelId,
}

#[derive(Default)]
pub struct PanelRegistry {
    messages: Mutex<HashMap<PanelKey, serenity::all::MessageId>>,
}

impl PanelRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn remember(&self, key: PanelKey, message_id: serenity::all::MessageId) {
        if let Ok(mut messages) = self.messages.lock() {
            messages.insert(key, message_id);
        }
    }

    pub async fn publish(
        &self,
        http: impl CacheHttp,
        key: PanelKey,
        embed: CreateEmbed,
        components: Vec<CreateActionRow>,
    ) -> serenity::Result<Message> {
        let existing = self
            .messages
            .lock()
            .ok()
            .and_then(|messages| messages.get(&key).copied());
        if let Some(message_id) = existing {
            if let Ok(message) = key
                .channel_id
                .edit_message(
                    &http,
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
                &http,
                CreateMessage::new().embed(embed).components(components),
            )
            .await?;
        if let Ok(mut messages) = self.messages.lock() {
            messages.insert(key, message.id);
        }
        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_keys_are_scoped_by_channel() {
        let first = PanelKey {
            scope: "lobby".into(),
            key: "7".into(),
            channel_id: ChannelId::new(1),
        };
        let second = PanelKey {
            channel_id: ChannelId::new(2),
            ..first.clone()
        };
        assert_ne!(first, second);
    }

    #[test]
    fn remembers_message_ids() {
        let registry = PanelRegistry::new();
        let key = PanelKey {
            scope: "lobby".into(),
            key: "7".into(),
            channel_id: ChannelId::new(1),
        };
        registry.remember(key.clone(), serenity::all::MessageId::new(42));
        assert_eq!(
            registry.messages.lock().unwrap().get(&key).copied(),
            Some(serenity::all::MessageId::new(42))
        );
    }
}
