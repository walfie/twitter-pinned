use crate::error::{BoxError, HttpError};
use crate::{Image, PinnedTweet, UserId};

use anyhow::Context as _;
use http::{Method, Request, Response};
use http_body::Body;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Service, ServiceExt};

#[derive(Debug)]
pub struct PinnedTweetService<S, B> {
    service: S,
    body: PhantomData<fn() -> B>,
}

impl<S, B> Clone for PinnedTweetService<S, B>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
            body: PhantomData,
        }
    }
}

impl<S, ReqB, RespB> PinnedTweetService<S, ReqB>
where
    S: Service<Request<ReqB>, Response = Response<RespB>> + Send + Clone,
{
    pub fn new(service: S) -> Self {
        Self {
            service,
            body: PhantomData,
        }
    }
}

impl<S, ReqB, RespB> Service<UserId> for PinnedTweetService<S, ReqB>
where
    S: Service<Request<ReqB>, Response = Response<RespB>> + Send + Clone + 'static,
    S::Error: Into<BoxError> + Send,
    S::Future: Send + 'static,
    ReqB: Body + Send + Default + 'static,
    RespB: Body + Send + 'static,
    RespB::Error: Into<BoxError>,
    RespB::Data: Send,
{
    type Response = Option<PinnedTweet>;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, user_id: UserId) -> Self::Future {
        let mut service = self.service.clone();

        // Use the service that was ready.
        // https://docs.rs/tower/0.4.13/tower/trait.Service.html#be-careful-when-cloning-inner-services
        std::mem::swap(&mut self.service, &mut service);

        Box::pin(async move {
            let req = Request::builder()
                .uri(graphql_url(user_id)?)
                .method(Method::POST)
                .body(Default::default())?;

            let resp = service
                .ready()
                .await
                .map_err(Into::into)?
                .call(req)
                .await
                .map_err(Into::into)?;

            // TODO: Status code checking could be moved into a tower layer
            let status = resp.status();
            let body = hyper::body::to_bytes(resp.into_body())
                .await
                .map_err(Into::into)?;

            if !status.is_success() {
                return Err(HttpError::new(status, body).into());
            }

            let json = serde_json::from_slice::<Value>(&body)?;
            Ok(extract_pinned_tweet_json(json)?)
        })
    }
}

fn graphql_url(user_id: UserId) -> anyhow::Result<String> {
    let mut url_encoder = form_urlencoded::Serializer::new(
        "https://api.twitter.com/graphql/urVlCWe1DTfZQbYRlTzxNA/UserTweets?".to_owned(),
    );

    let variables = serde_json::to_string(&GetUserTweets {
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
    })?;

    let params = QueryString { variables };

    params.serialize(serde_urlencoded::Serializer::new(&mut url_encoder))?;
    Ok(url_encoder.finish())
}

fn extract_pinned_tweet_json(json: Value) -> anyhow::Result<Option<PinnedTweet>> {
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

#[derive(Serialize)]
pub struct QueryString {
    variables: String,
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
