use anyhow::Result;
use argh::FromArgs;
use dotenvy::dotenv;
use tracing::info;
use sqlx::postgres::PgPoolOptions;
use std::env;

use lib::models::feed::{CreateFeed, FeedType};
use lib::commands::add_feed::add_feed;

#[derive(FromArgs)]
/// CLI for crawlect
struct Args {
    #[argh(subcommand)]
    commands: Commands,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Commands {
    AddFeed(AddFeed),
}

#[derive(FromArgs)]
/// Add a feed to the database
#[argh(subcommand, name = "add-feed")]
struct AddFeed {
    #[argh(option)]
    /// title of the feed (max 255 characters)
    title: String,
    #[argh(option)]
    /// URL of the feed (max 2048 characters)
    url: String,
    #[argh(option, long = "type")]
    /// type of the feed ('rss' or 'atom')
    feed_type: FeedType,
    #[argh(option)]
    /// description of the feed
    description: Option<String>,
}


#[tokio::main]
pub async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt::init();

    let pool = PgPoolOptions::new()
        .max_connections(env::var("DATABASE_MAX_CONNECTIONS")?.parse()?)
        .connect(&env::var("DATABASE_URL")?)
        .await?;

    let args: Args = argh::from_env();

    if let Commands::AddFeed(add_feed_args) = args.commands {
        add_feed(pool, CreateFeed {
            title: add_feed_args.title,
            url: add_feed_args.url,
            feed_type: add_feed_args.feed_type,
            description: add_feed_args.description,
        }).await?;
    }

    Ok(())
}
