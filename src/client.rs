use anyhow::{Context, Result};
use reqwest::header::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_USER_AGENT: &'static str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/94.0.4606.71 Safari/537.36";

pub const DEFAULT_BEARER_TOKEN: &'static str = "AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";

#[derive(Debug)]
pub struct Client {
    client: reqwest::Client,
    headers: HeaderMap,
}

impl Client {
    pub async fn build(user_agent: String, bearer_token: String) -> Result<Self> {
        let req_client = reqwest::Client::builder()
            .user_agent(user_agent)
            .build()
            .context("failed to create client")?;

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", bearer_token)
                .parse()
                .context("failed to parse bearer token")?,
        );

        let mut client = Self {
            client: req_client,
            headers,
        };

        client.get_token().await?;
        Ok(client)
    }

    pub async fn build_default() -> Result<Self> {
        Self::build(
            DEFAULT_USER_AGENT.to_owned(),
            DEFAULT_BEARER_TOKEN.to_owned(),
        )
        .await
    }

    pub async fn get_pinned_tweet(&self, user_id: u64) -> Result<Option<PinnedTweet>> {
        let json: Value = self
            .client
            .post("https://api.twitter.com/graphql/urVlCWe1DTfZQbYRlTzxNA/UserTweets")
            .query(&[(
                "variables",
                &serde_json::to_string(&GetUserTweets {
                    user_id: user_id.to_string(),
                    count: 1,
                    with_tweet_quote_count: false,
                    include_promoted_content: false,
                    with_super_follows_user_fields: false,
                    with_user_results: false,
                    with_birdwatch_pivots: false,
                    with_reactions_metadata: false,
                    with_reactions_perspective: false,
                    with_super_follows_tweet_fields: false,
                    with_voice: false,
                })?,
            )])
            .headers(self.headers.clone())
            .send()
            .await?
            .json::<Value>()
            .await?;

        let result = json
            .pointer("/data/user/result/timeline/timeline/instructions")
            .context("failed to get timeline instructions")?
            .as_array()
            .context("failed to parse instructions as array")?
            .iter()
            .find_map(|value| match value.pointer("/type") {
                Some(Value::String(s)) if s == "TimelinePinEntry" => {
                    value.pointer("/entry/content/itemContent/tweet_results/result")
                }
                _ => None,
            });

        let json = match result {
            None => return Ok(None),
            Some(json) => json,
        };

        let tweet = serde_json::from_value::<Legacy>(
            json.pointer("/legacy")
                .context("failed to find legacy field")?
                .clone(),
        )
        .context("failed to parse pinned tweet")?;

        let screen_name = match json.pointer("/core/user/legacy/screen_name") {
            Some(Value::String(s)) => s.to_owned(),
            _ => anyhow::bail!("failed to find screen_name in pinned tweet"),
        };

        Ok(Some(PinnedTweet {
            screen_name,
            created_at: tweet.created_at,
            tweet_id: tweet.id_str,
            user_id: tweet.user_id_str,
            text: tweet.full_text,
            images: tweet
                .entities
                .media
                .unwrap_or(Vec::new())
                .into_iter()
                .map(|media| Image {
                    url: media.media_url_https,
                    width: media.sizes.large.w,
                    height: media.sizes.large.h,
                })
                .collect(),
        }))
    }

    async fn get_token(&mut self) -> Result<()> {
        let token = self
            .client
            .post("https://api.twitter.com/1.1/guest/activate.json")
            .headers(self.headers.clone())
            .send()
            .await
            .context("failed to call Twitter Activate API")?
            .json::<Activate>()
            .await
            .context("failed to parse response of Twitter Activate API")?
            .guest_token;

        self.headers.insert("X-Guest-Token", token.parse()?);
        Ok(())
    }
}

#[derive(Deserialize)]
pub struct Activate {
    pub guest_token: String,
}

#[derive(Serialize)]
pub struct UserByScreenName {
    screen_name: String,
    #[serde(rename = "withHighlightedLabel")]
    with_highlighted_label: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUserTweets {
    user_id: String,
    count: usize,
    with_tweet_quote_count: bool,
    include_promoted_content: bool,
    with_super_follows_user_fields: bool,
    with_user_results: bool,
    with_birdwatch_pivots: bool,
    with_reactions_metadata: bool,
    with_reactions_perspective: bool,
    with_super_follows_tweet_fields: bool,
    with_voice: bool,
}

#[derive(Debug, Serialize)]
pub struct PinnedTweet {
    screen_name: String,
    created_at: String,
    tweet_id: String,
    user_id: String,
    text: String,
    images: Vec<Image>,
}

#[derive(Debug, Serialize)]
pub struct Image {
    url: String,
    width: usize,
    height: usize,
}

#[derive(Deserialize, Debug)]
pub struct Legacy {
    pub id_str: String,
    pub user_id_str: String,
    pub full_text: String,
    pub created_at: String,
    pub entities: Entities,
}

#[derive(Deserialize, Debug)]
pub struct Entities {
    pub media: Option<Vec<Media>>,
}

#[derive(Deserialize, Debug)]
pub struct Media {
    pub media_url_https: String,
    pub sizes: Sizes,
}

#[derive(Deserialize, Debug)]
pub struct Sizes {
    large: Size,
}

#[derive(Deserialize, Debug)]
pub struct Size {
    h: usize,
    w: usize,
}
