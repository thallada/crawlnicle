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
use crate::partials::opml_import_form::opml_import_form;
use crate::state::Imports;
use crate::uuid::Base62Uuid;

pub async fn opml(
    State(imports): State<Imports>,
    State(importer): State<ImporterHandle>,
    mut multipart: Multipart,
) -> Result<Response> {
    if let Some(field) = multipart.next_field().await? {
        let import_id = Base62Uuid::new();
        let file_name = field.file_name().map(|s| s.to_string());
        dbg!(&file_name);
        let bytes = field.bytes().await?;
        dbg!(bytes.len());
        let receiver = importer.import(import_id.as_uuid(), file_name, bytes).await;
        {
            let mut imports = imports.lock().await;
            imports.insert(import_id.as_uuid(), receiver);
        }

        let import_stream = format!("connnect:/import/{}/stream", import_id);
        return Ok((
            StatusCode::CREATED,
            html! {
                (opml_import_form())
                div hx-sse=(import_stream) {
                    ul class="stream-messages" hx-sse="swap:message" hx-swap="beforeend" {
                        li { "Uploading..."}
                    }
                }
            }
            .into_string(),
        )
            .into_response());
    }
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
    let stream = stream.map(move |msg| match msg {
        Ok(ImporterHandleMessage::Import(Ok(_))) => Ok::<Event, String>(
            Event::default().data(
                html! {
                    li { "Finished importing" }
                }
                .into_string(),
            ),
        ),
        Ok(ImporterHandleMessage::CrawlScheduler(CrawlSchedulerHandleMessage::FeedCrawler(
            FeedCrawlerHandleMessage::Entry(Ok(entry)),
        ))) => Ok::<Event, String>(
            Event::default().data(
                html! {
                    li { "Crawled entry: " (entry_link(entry)) }
                }
                .into_string(),
            ),
        ),
        Ok(ImporterHandleMessage::CrawlScheduler(CrawlSchedulerHandleMessage::FeedCrawler(
            FeedCrawlerHandleMessage::Feed(Ok(feed)),
        ))) => Ok::<Event, String>(
            Event::default().data(
                html! {
                    li { "Crawled feed: " (feed_link(&feed, false)) }
                }
                .into_string(),
            ),
        ),
        Ok(ImporterHandleMessage::CrawlScheduler(CrawlSchedulerHandleMessage::FeedCrawler(
            FeedCrawlerHandleMessage::Feed(Err(error)),
        ))) => Ok::<Event, String>(
            Event::default().data(
                html! {
                    li { span class="error" { (error) } }
                }
                .into_string(),
            ),
        ),
        Ok(ImporterHandleMessage::CrawlScheduler(CrawlSchedulerHandleMessage::FeedCrawler(
            FeedCrawlerHandleMessage::Entry(Err(error)),
        ))) => Ok::<Event, String>(
            Event::default().data(
                html! {
                    li { span class="error" { (error) } }
                }
                .into_string(),
            ),
        ),
        Ok(ImporterHandleMessage::CrawlScheduler(CrawlSchedulerHandleMessage::Schedule(Err(
            error,
        )))) => Ok::<Event, String>(
            Event::default().data(
                html! {
                    li { span class="error" { (error) } }
                }
                .into_string(),
            ),
        ),
        Ok(ImporterHandleMessage::CreateFeedError(url)) => Ok::<Event, String>(
            Event::default().data(
                html! {
                    li {
                        span class="error" {
                            "Could not create feed for url: " a href=(url) { (url) }
                        }
                    }
                }
                .into_string(),
            ),
        ),
        Ok(ImporterHandleMessage::Import(Err(error))) => Ok(Event::default().data(
            html! {
                li { span class="error" { (error) } }
            }
            .into_string(),
        )),
        Ok(ImporterHandleMessage::AlreadyImported(url)) => Ok(Event::default().data(
            html! {
                li { "Already imported feed: " a href=(url) { (url) } }
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
