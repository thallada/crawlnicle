use std::io::Cursor;

use anyhow::{anyhow, Context, Result};
use apalis::prelude::*;
use apalis_redis::RedisStorage;
use fred::prelude::*;
use opml::OPML;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{error, instrument, warn};
use uuid::Uuid;

use crate::error::Error;
use crate::jobs::crawl_feed::CrawlFeedJob;
use crate::jobs::AsyncJob;
use crate::models::feed::{CreateFeed, Feed};
use crate::uuid::Base62Uuid;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ImportOpmlJob {
    pub import_id: Uuid,
    pub file_name: Option<String>,
    pub bytes: Vec<u8>,
}

// TODO: send messages over redis channel
/// `ImporterOpmlMessage::Import` contains the result of importing the OPML file.
// #[allow(clippy::large_enum_variant)]
// #[derive(Debug, Clone)]
// pub enum ImporterOpmlMessage {
//     Import(ImporterResult<()>),
//     CreateFeedError(String),
//     AlreadyImported(String),
//     CrawlScheduler(CrawlSchedulerHandleMessage),
// }

#[instrument(skip_all, fields(import_id = %import_id))]
pub async fn import_opml(
    ImportOpmlJob {
        import_id,
        file_name,
        bytes,
    }: ImportOpmlJob,
    db: Data<PgPool>,
    apalis: Data<RedisStorage<AsyncJob>>,
    redis: Data<RedisPool>,
) -> Result<()> {
    let document = OPML::from_reader(&mut Cursor::new(bytes)).with_context(|| {
        format!(
            "Failed to read OPML file for import {} from file {}",
            Base62Uuid::from(import_id),
            file_name
                .map(|n| n.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        )
    })?;
    for url in gather_feed_urls(document.body.outlines) {
        let feed = Feed::create(
            &*db,
            CreateFeed {
                url: url.clone(),
                ..Default::default()
            },
        )
        .await;
        match feed {
            Ok(feed) => {
                (*apalis)
                    .clone()
                    .push(AsyncJob::CrawlFeed(CrawlFeedJob {
                        feed_id: feed.feed_id,
                    }))
                    .await?;
            }
            Err(Error::Sqlx(sqlx::error::Error::Database(err))) => {
                if err.is_unique_violation() {
                    // let _ = respond_to.send(ImporterHandleMessage::AlreadyImported(url));
                    warn!("Feed {} already imported", url);
                }
            }
            Err(err) => {
                // let _ = respond_to.send(ImporterHandleMessage::CreateFeedError(url));
                error!("Failed to create feed for {}", url);
                return Err(anyhow!(err));
            }
        }
    }

    redis
        .next()
        .publish("imports", import_id.to_string())
        .await?;
    Ok(())
}

fn gather_feed_urls(outlines: Vec<opml::Outline>) -> Vec<String> {
    let mut urls = Vec::new();
    for outline in outlines.into_iter() {
        if let Some(url) = outline.xml_url {
            urls.push(url);
        }
        urls.append(&mut gather_feed_urls(outline.outlines));
    }
    urls
}
