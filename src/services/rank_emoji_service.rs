use crate::models::Rank;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

const DISCORD_API: &str = "https://discord.com/api/v10";
const ICON_BASE: &str =
    "https://raw.githubusercontent.com/odota/web/master/public/assets/images/dota2/rank_icons";
const EMOJI_PREFIX: &str = "dota_rank_";

#[derive(Debug, Clone, Default)]
pub struct RankEmojiMap(HashMap<Rank, String>);

impl RankEmojiMap {
    pub fn label(&self, rank: Rank) -> String {
        self.0
            .get(&rank)
            .map(|emoji| format!("{emoji} {}", rank.badge()))
            .unwrap_or_else(|| rank.badge().to_string())
    }
}

#[derive(Debug, Deserialize)]
struct CurrentApplication {
    id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ApplicationEmoji {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct ApplicationEmojiList {
    items: Vec<ApplicationEmoji>,
}

#[derive(Debug, Serialize)]
struct CreateEmojiRequest<'a> {
    name: &'a str,
    image: String,
}

pub async fn ensure(token: &str) -> RankEmojiMap {
    let client = match Client::builder().timeout(Duration::from_secs(10)).build() {
        Ok(client) => client,
        Err(error) => {
            tracing::warn!(error = %error, "could not initialize Discord rank emoji client");
            return RankEmojiMap::default();
        }
    };
    let application = match get_json::<CurrentApplication>(&client, token, "/applications/@me")
        .await
    {
        Ok(application) => application,
        Err(error) => {
            tracing::warn!(error = %error, "could not resolve Discord application for rank emojis");
            return RankEmojiMap::default();
        }
    };
    let path = format!("/applications/{}/emojis", application.id);
    let mut emojis = match get_json::<ApplicationEmojiList>(&client, token, &path).await {
        Ok(emojis) => emojis.items,
        Err(error) => {
            tracing::warn!(error = %error, "could not list Discord application emojis");
            return RankEmojiMap::default();
        }
    };

    for (icon_index, rank) in Rank::all().into_iter().enumerate() {
        let name = format!("{EMOJI_PREFIX}{}", rank.emoji_tier_name());
        if emojis.iter().any(|emoji| emoji.name == name) {
            continue;
        }
        match create_emoji(&client, token, &application.id, &name, icon_index).await {
            Ok(emoji) => emojis.push(emoji),
            Err(error) => {
                tracing::warn!(rank = %rank, error = %error, "could not provision Discord rank emoji")
            }
        }
    }

    RankEmojiMap(
        emojis
            .into_iter()
            .filter_map(|emoji| {
                let tier = emoji.name.strip_prefix(EMOJI_PREFIX)?;
                let rank = Rank::from_emoji_tier_name(tier)?;
                Some((rank, format!("<:{}:{}>", emoji.name, emoji.id)))
            })
            .collect(),
    )
}

async fn get_json<T: for<'de> Deserialize<'de>>(
    client: &Client,
    token: &str,
    path: &str,
) -> Result<T, String> {
    let response = client
        .get(format!("{DISCORD_API}{path}"))
        .header("Authorization", format!("Bot {token}"))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("Discord returned {status}"));
    }
    response.json().await.map_err(|error| error.to_string())
}

async fn create_emoji(
    client: &Client,
    token: &str,
    application_id: &str,
    name: &str,
    icon_index: usize,
) -> Result<ApplicationEmoji, String> {
    let icon_url = format!("{ICON_BASE}/rank_icon_{icon_index}.png");
    let icon_response = client
        .get(icon_url)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if icon_response.status() != StatusCode::OK {
        return Err(format!("icon download returned {}", icon_response.status()));
    }
    let image = STANDARD.encode(
        icon_response
            .bytes()
            .await
            .map_err(|error| error.to_string())?,
    );
    let response = client
        .post(format!(
            "{DISCORD_API}/applications/{application_id}/emojis"
        ))
        .header("Authorization", format!("Bot {token}"))
        .json(&CreateEmojiRequest {
            name,
            image: format!("data:image/png;base64,{image}"),
        })
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("Discord returned {status}"));
    }
    response.json().await.map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_names_round_trip() {
        for rank in Rank::all() {
            assert_eq!(
                Rank::from_emoji_tier_name(rank.emoji_tier_name()),
                Some(rank)
            );
        }
    }
}
