use anyhow::{Context, Result};
use structopt::StructOpt;
use twitter_pinned::*;

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

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::from_args();

    let mut attempt = 0;

    loop {
        if let Err(e) = run(&opt).await {
            if attempt < opt.retry {
                attempt += 1;
                eprintln!("Error: {:?}\n", e);
                eprintln!(
                    "Retrying on failure (reattempt {} of {})",
                    attempt, opt.retry
                );
            } else {
                return Err(e);
            }
        } else {
            return Ok(());
        }
    }
}

async fn run(opt: &Opt) -> Result<()> {
    let client = Client::build(opt.user_agent.clone(), opt.bearer_token.clone()).await?;

    let reqs = opt.user_ids.iter().map(|id| {
        let req = client.get_pinned_tweet(*id);
        async move {
            req.await
                .with_context(|| format!("failed to get pinned tweet for user {}", id))
        }
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
