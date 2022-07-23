use serde::Serialize;

pub mod error;
pub mod service;

pub const DEFAULT_USER_AGENT: &'static str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/94.0.4606.71 Safari/537.36";

pub const DEFAULT_BEARER_TOKEN: &'static str = "AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";

pub type UserId = u64;

#[derive(Debug, Serialize)]
pub struct PinnedTweet {
    pub screen_name: String,
    pub created_at: String,
    pub tweet_id: String,
    pub user_id: String,
    pub text: String,
    pub images: Vec<Image>,
}

#[derive(Debug, Serialize)]
pub struct Image {
    pub url: String,
    pub width: usize,
    pub height: usize,
}
