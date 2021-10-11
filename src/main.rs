use anyhow::{Context, Result};
use structopt::StructOpt;
use twitter_pinned::*;

#[derive(StructOpt)]
pub struct Opt {
    #[structopt(long, env, default_value = DEFAULT_USER_AGENT)]
    pub user_agent: String,
    #[structopt(long, env, default_value = DEFAULT_BEARER_TOKEN)]
    pub bearer_token: String,
    #[structopt(required = true)]
    /// Twitter user IDs
    pub user_ids: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::from_args();

    let client = Client::build(opt.user_agent.clone(), opt.bearer_token.clone()).await?;

    let reqs = opt.user_ids.into_iter().map(|id| async {
        client
            .get_pinned_tweet(id.clone())
            .await
            .with_context(move || format!("failed to get pinned tweet for user {}", &id))
    });

    let tweets = futures::future::try_join_all(reqs)
        .await
        .context("failed to get pinned tweet")?;

    println!("{}", serde_json::to_string_pretty(&tweets)?);

    Ok(())
}
