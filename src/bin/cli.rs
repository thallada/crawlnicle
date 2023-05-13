use anyhow::Result;
use argh::FromArgs;
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tracing::info;

use lib::jobs::crawl::crawl;
use lib::models::feed::{create_feed, delete_feed, CreateFeed, FeedType};
use lib::models::entry::{create_entry, delete_entry, CreateEntry};

#[derive(FromArgs)]
/// CLI for crawlnicle
struct Args {
    #[argh(subcommand)]
    commands: Commands,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Commands {
    AddFeed(AddFeed),
    DeleteFeed(DeleteFeed),
    AddEntry(AddEntry),
    DeleteEntry(DeleteEntry),
    Crawl(Crawl),
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

#[derive(FromArgs)]
/// Delete a feed from the database
#[argh(subcommand, name = "delete-feed")]
struct DeleteFeed {
    #[argh(positional)]
    /// id of the feed to delete
    id: i32,
}

#[derive(FromArgs)]
/// Add an entry to the database
#[argh(subcommand, name = "add-entry")]
struct AddEntry {
    #[argh(option)]
    /// title of the entry (max 255 characters)
    title: String,
    #[argh(option)]
    /// URL of the entry (max 2048 characters)
    url: String,
    #[argh(option)]
    /// description of the entry
    description: Option<String>,
    #[argh(option)]
    /// source feed for the entry
    feed_id: i32,
}

#[derive(FromArgs)]
/// Delete an entry from the database
#[argh(subcommand, name = "delete-entry")]
struct DeleteEntry {
    #[argh(positional)]
    /// id of the entry to delete
    id: i32,
}

#[derive(FromArgs)]
/// Delete an entry from the database
#[argh(subcommand, name = "crawl")]
struct Crawl {}

#[tokio::main]
pub async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt::init();

    let pool = PgPoolOptions::new()
        .max_connections(env::var("DATABASE_MAX_CONNECTIONS")?.parse()?)
        .connect(&env::var("DATABASE_URL")?)
        .await?;

    let args: Args = argh::from_env();

    info!("hello?");

    match args.commands {
        Commands::AddFeed(args) => {
            let feed = create_feed(
                &pool,
                CreateFeed {
                    title: args.title,
                    url: args.url,
                    feed_type: args.feed_type,
                    description: args.description,
                },
            )
            .await?;
            info!("Created feed with id {}", feed.id);
        }
        Commands::DeleteFeed(args) => {
            delete_feed(&pool, args.id).await?;
            info!("Deleted feed with id {}", args.id);
        }
        Commands::AddEntry(args) => {
            let entry = create_entry(
                &pool,
                CreateEntry {
                    title: args.title,
                    url: args.url,
                    description: args.description,
                    feed_id: args.feed_id,
                },
            )
            .await?;
            info!("Created entry with id {}", entry.id);
        }
        Commands::DeleteEntry(args) => {
            delete_entry(&pool, args.id).await?;
            info!("Deleted entry with id {}", args.id);
        }
        Commands::Crawl(_) => {
            info!("Crawling...");
            crawl(&pool).await?;
        }
    }

    Ok(())
}
