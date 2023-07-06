use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use chrono::Utc;
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tracing::info;
use uuid::Uuid;

use lib::jobs::crawl::crawl;
use lib::models::feed::{create_feed, delete_feed, CreateFeed, FeedType};
use lib::models::entry::{create_entry, delete_entry, CreateEntry};
use lib::uuid::Base62Uuid;

/// CLI for crawlnicle
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetches new entries from all feeds in the database
    Crawl,
    AddFeed(AddFeed),
    DeleteFeed(DeleteFeed),
    AddEntry(AddEntry),
    DeleteEntry(DeleteEntry),
}

/// Add a feed to the database
#[derive(Args)]
struct AddFeed {
    /// title of the feed (max 255 characters)
    #[arg(short, long)]
    title: Option<String>,
    /// URL of the feed (max 2048 characters)
    #[arg(short, long)]
    url: String,
    /// type of the feed ('rss' or 'atom')
    #[arg(short, long)]
    feed_type: FeedType,
    /// description of the feed
    #[arg(short, long)]
    description: Option<String>,
}

#[derive(Args)]
/// Delete a feed from the database
struct DeleteFeed {
    /// id of the feed to delete
    id: Uuid,
}

#[derive(Args)]
/// Add an entry to the database
struct AddEntry {
    /// title of the entry (max 255 characters)
    #[arg(short, long)]
    title: Option<String>,
    /// URL of the entry (max 2048 characters)
    #[arg(short, long)]
    url: String,
    /// description of the entry
    #[arg(short, long)]
    description: Option<String>,
    /// source feed for the entry
    #[arg(short, long)]
    feed_id: Uuid,
}

#[derive(Args)]
/// Delete an entry from the database
struct DeleteEntry {
    /// id of the entry to delete
    id: Uuid,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt::init();

    let pool = PgPoolOptions::new()
        .max_connections(env::var("DATABASE_MAX_CONNECTIONS")?.parse()?)
        .connect(&env::var("DATABASE_URL")?)
        .await?;

    let cli: Cli = Cli::parse();

    match cli.commands {
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
            info!("Created feed with id {}", Base62Uuid::from(feed.feed_id));
        }
        Commands::DeleteFeed(args) => {
            delete_feed(&pool, args.id).await?;
            info!("Deleted feed with id {}", Base62Uuid::from(args.id));
        }
        Commands::AddEntry(args) => {
            let entry = create_entry(
                &pool,
                CreateEntry {
                    title: args.title,
                    url: args.url,
                    description: args.description,
                    feed_id: args.feed_id,
                    published_at: Utc::now(),
                },
            )
            .await?;
            info!("Created entry with id {}", Base62Uuid::from(entry.entry_id));
        }
        Commands::DeleteEntry(args) => {
            delete_entry(&pool, args.id).await?;
            info!("Deleted entry with id {}", Base62Uuid::from(args.id));
        }
        Commands::Crawl => {
            info!("Crawling...");
            crawl(&pool).await?;
        }
    }

    Ok(())
}
