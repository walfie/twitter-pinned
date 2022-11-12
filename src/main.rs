use structopt::StructOpt;

use twitter_pinned::error::BoxError;
use twitter_pinned::service::{GuestTokenService, PinnedTweetService, RetryOnHttpError};
use twitter_pinned::{DEFAULT_BEARER_TOKEN, DEFAULT_USER_AGENT};

use anyhow::{anyhow, Context, Result};
use futures::TryFutureExt;
use http::Request;
use hyper::Body;
use std::time::Duration;
use tracing::Span;

use hyper_tls::HttpsConnector;
use tower::{Service, ServiceBuilder, ServiceExt};
use tower_http::decompression::DecompressionLayer;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    tracing_subscriber::fmt::fmt()
        .with_writer(std::io::stderr)
        .init();

    let opt = Opt::from_args();

    let https = HttpsConnector::new();

    let service = GuestTokenService::new(
        hyper::Client::builder().build::<_, hyper::Body>(https),
        opt.user_agent,
        opt.bearer_token,
    )?;

    let trace_layer = TraceLayer::new_for_http().on_request(|req: &Request<Body>, _span: &Span| {
        tracing::debug!("Sending request {} {}", req.method(), req.uri());
    });

    let service = ServiceBuilder::new()
        .layer(trace_layer)
        .layer(DecompressionLayer::new())
        .timeout(Duration::from_secs(5))
        .service(service);

    let service = ServiceBuilder::new()
        .retry(RetryOnHttpError::new(opt.retry))
        .service(PinnedTweetService::new(service));

    let reqs = opt.user_ids.iter().map(|id| {
        let mut service = service.clone();
        async move { service.ready().await?.call(*id).await }.map_err(move |e| {
            anyhow!(e).context(format!("failed to get pinned tweet for user {}", id))
        })
    });

    let tweets = futures::future::try_join_all(reqs)
        .await
        .context("failed to get pinned tweet")?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let out = if opt.pretty {
        serde_json::to_string_pretty(&tweets)?
    } else {
        serde_json::to_string(&tweets)?
    };

    println!("{}", out);

    Ok(())
}

#[derive(StructOpt)]
pub struct Opt {
    #[structopt(long, env, default_value = DEFAULT_USER_AGENT)]
    pub user_agent: String,
    #[structopt(long, env, default_value = DEFAULT_BEARER_TOKEN)]
    pub bearer_token: String,

    /// Pretty print JSON output
    #[structopt(long)]
    pub pretty: bool,

    /// Twitter user IDs
    #[structopt(required = true)]
    pub user_ids: Vec<u64>,

    /// Number of times to retry on failure
    #[structopt(long, env, default_value = "0")]
    pub retry: usize,
}
