use anyhow::Result;
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use dotenvy::dotenv;
use lib::actors::feed_crawler::FeedCrawlerHandle;
use lib::domain_locks::DomainLocks;
use reqwest::Client;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tracing::info;
use uuid::Uuid;

use lib::models::entry::{CreateEntry, Entry};
use lib::models::feed::{CreateFeed, Feed, FeedType};
use lib::uuid::Base62Uuid;
use lib::USER_AGENT;

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
    Crawl(CrawlFeed),
    AddFeed(AddFeed),
    DeleteFeed(DeleteFeed),
    AddEntry(AddEntry),
    DeleteEntry(DeleteEntry),
}

#[derive(Args)]
/// Crawl a feed (get new entries)
struct CrawlFeed {
    /// id of the feed to crawl
    id: Uuid,
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
    let crawls = Arc::new(Mutex::new(HashMap::new()));

    let cli: Cli = Cli::parse();

    match cli.commands {
        Commands::AddFeed(args) => {
            let feed = Feed::create(
                &pool,
                CreateFeed {
                    title: args.title,
                    url: args.url,
                    description: args.description,
                },
            )
            .await?;
            info!("Created feed with id {}", Base62Uuid::from(feed.feed_id));
        }
        Commands::DeleteFeed(args) => {
            Feed::delete(&pool, args.id).await?;
            info!("Deleted feed with id {}", Base62Uuid::from(args.id));
        }
        Commands::AddEntry(args) => {
            let entry = Entry::create(
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
            Entry::delete(&pool, args.id).await?;
            info!("Deleted entry with id {}", Base62Uuid::from(args.id));
        }
        Commands::Crawl(CrawlFeed { id }) => {
            info!("Crawling feed {}...", Base62Uuid::from(id));
            let client = Client::builder().user_agent(USER_AGENT).build()?;
            // NOTE: this is not the same DomainLocks as the one used in the server so, if the
            // server is running, it will *not* serialize same-domain requests with it.
            let domain_locks = DomainLocks::new();
            let feed_crawler = FeedCrawlerHandle::new(
                pool.clone(),
                client.clone(),
                domain_locks.clone(),
                env::var("CONTENT_DIR")?,
                crawls.clone(),
            );
            let _ = feed_crawler.crawl(id).await;
        }
    }

    Ok(())
}
