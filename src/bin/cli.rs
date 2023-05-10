use anyhow::Result;
use argh::FromArgs;
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tracing::info;

use lib::jobs::crawl::crawl;
use lib::models::feed::{create_feed, delete_feed, CreateFeed, FeedType};
use lib::models::item::{create_item, delete_item, CreateItem};

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
    DeleteFeed(DeleteFeed),
    AddItem(AddItem),
    DeleteItem(DeleteItem),
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
/// Add an item to the database
#[argh(subcommand, name = "add-item")]
struct AddItem {
    #[argh(option)]
    /// title of the item (max 255 characters)
    title: String,
    #[argh(option)]
    /// URL of the item (max 2048 characters)
    url: String,
    #[argh(option)]
    /// description of the item
    description: Option<String>,
    #[argh(option)]
    /// source feed for the item
    feed_id: i32,
}

#[derive(FromArgs)]
/// Delete an item from the database
#[argh(subcommand, name = "delete-item")]
struct DeleteItem {
    #[argh(positional)]
    /// id of the item to delete
    id: i32,
}

#[derive(FromArgs)]
/// Delete an item from the database
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
        Commands::AddItem(args) => {
            let item = create_item(
                &pool,
                CreateItem {
                    title: args.title,
                    url: args.url,
                    description: args.description,
                    feed_id: args.feed_id,
                },
            )
            .await?;
            info!("Created item with id {}", item.id);
        }
        Commands::DeleteItem(args) => {
            delete_item(&pool, args.id).await?;
            info!("Deleted item with id {}", args.id);
        }
        Commands::Crawl(_) => {
            info!("Crawling...");
            crawl(&pool).await?;
        }
    }

    Ok(())
}
