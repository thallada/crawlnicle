use std::time::Duration;

use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive};
use axum::response::{IntoResponse, Response, Sse};
use maud::html;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::actors::crawl_scheduler::CrawlSchedulerHandleMessage;
use crate::actors::feed_crawler::FeedCrawlerHandleMessage;
use crate::actors::importer::{ImporterHandle, ImporterHandleMessage};
use crate::error::{Error, Result};
use crate::partials::entry_link::entry_link;
use crate::partials::feed_link::feed_link;
use crate::state::Imports;
use crate::turbo_stream::TurboStream;
use crate::uuid::Base62Uuid;

pub async fn opml(
    State(imports): State<Imports>,
    State(importer): State<ImporterHandle>,
    mut multipart: Multipart,
) -> Result<Response> {
    dbg!("opml handler");
    if let Some(field) = multipart.next_field().await.map_err(|err| {
        dbg!(&err);
        err
    })? {
        let import_id = Base62Uuid::new();
        dbg!(&import_id);
        let file_name = field.file_name().map(|s| s.to_string());
        dbg!(&file_name);
        let bytes = field.bytes().await?;
        dbg!(&bytes.len());
        let receiver = importer.import(import_id, file_name, bytes).await;
        {
            let mut imports = imports.lock().await;
            imports.insert(import_id.as_uuid(), receiver);
        }

        let import_html_id = format!("import-{}", import_id);
        let import_stream = format!("/import/{}/stream", import_id);
        return Ok((
            StatusCode::CREATED,
            TurboStream(
                html! {
                    turbo-stream-source src=(import_stream) id="import-stream" {}
                    turbo-stream action="append" target="add-feed-messages" {
                        template {
                            li { "Uploading file..." }
                        }
                    }
                    turbo-stream action="remove" target="no-feeds";
                }
                .into_string(),
            ),
        )
            .into_response());
    }
    dbg!("no file");
    Err(Error::NoFile)
}

pub async fn stream(
    Path(id): Path<Base62Uuid>,
    State(imports): State<Imports>,
) -> Result<impl IntoResponse> {
    let receiver = {
        let mut imports = imports.lock().await;
        imports.remove(&id.as_uuid())
    }
    .ok_or_else(|| Error::NotFound("import stream", id.as_uuid()))?;

    let stream = BroadcastStream::new(receiver);
    let import_html_id = format!("import-{}", id);
    let stream = stream.map(move |msg| match msg {
        Ok(ImporterHandleMessage::Import(Ok(_))) => Ok::<Event, String>(
            Event::default().data(
                html! {
                    turbo-stream action="append" target="add-feed-messages" {
                        template { li { "Importing...." } }
                    }
                }
                .into_string(),
            ),
        ),
        Ok(ImporterHandleMessage::CrawlScheduler(CrawlSchedulerHandleMessage::FeedCrawler(
            FeedCrawlerHandleMessage::Entry(Ok(entry)),
        ))) => Ok::<Event, String>(
            Event::default().data(
                html! {
                    turbo-stream action="append" target="add-feed-messages" {
                        template {
                            li { "Imported: " (entry_link(entry)) }
                        }
                    }
                }
                .into_string(),
            ),
        ),
        Ok(ImporterHandleMessage::CrawlScheduler(CrawlSchedulerHandleMessage::FeedCrawler(
            FeedCrawlerHandleMessage::Feed(Ok(feed)),
        ))) => Ok::<Event, String>(
            Event::default().data(
                html! {
                    turbo-stream action="remove" target="import-stream" {}
                    turbo-stream action="append" target="add-feed-messages" {
                        template {
                            li { "Finished import." }
                        }
                    }
                    turbo-stream action="prepend" target="feeds" {
                        template {
                            li id=(format!("feed-{}", feed.feed_id)) { (feed_link(&feed, false)) }
                        }
                    }
                }
                .into_string(),
            ),
        ),
        Ok(ImporterHandleMessage::CrawlScheduler(CrawlSchedulerHandleMessage::FeedCrawler(
            FeedCrawlerHandleMessage::Feed(Err(error)),
        ))) => Ok::<Event, String>(
            Event::default().data(
                html! {
                    turbo-stream action="append" target="add-feed-messages" {
                        template {
                            li { span class="error" { (error) } }
                        }
                    }
                }
                .into_string(),
            ),
        ),
        Ok(ImporterHandleMessage::CrawlScheduler(CrawlSchedulerHandleMessage::FeedCrawler(
            FeedCrawlerHandleMessage::Entry(Err(error)),
        ))) => Ok::<Event, String>(
            Event::default().data(
                html! {
                    turbo-stream action="append" target="add-feed-messages" {
                        template {
                            li { span class="error" { (error) } }
                        }
                    }
                }
                .into_string(),
            ),
        ),
        Ok(ImporterHandleMessage::Import(Err(error))) => Ok(Event::default().data(
            html! {
                turbo-stream action="remove" target="import-stream" {}
                turbo-stream action="append" target="add-feed-messages" {
                    template {
                        li { span class="error" { (error) } }
                    }
                }
                turbo-stream action="replace" target=(import_html_id) {
                    template {
                        li id=(import_html_id) { span class="error" { (error) } }
                    }
                }
            }
            .into_string(),
        )),
        _ => Ok(Event::default()),
    });
    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive-text"),
    ))
}
