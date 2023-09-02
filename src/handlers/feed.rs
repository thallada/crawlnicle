use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive};
use axum::response::{IntoResponse, Redirect, Response, Sse};
use axum::Form;
use feed_rs::parser;
use maud::html;
use serde::Deserialize;
use serde_with::{serde_as, NoneAsEmptyString};
use sqlx::PgPool;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::actors::crawl_scheduler::{CrawlSchedulerHandle, CrawlSchedulerHandleMessage};
use crate::actors::feed_crawler::FeedCrawlerHandleMessage;
use crate::error::{Error, Result};
use crate::models::entry::{Entry, GetEntriesOptions};
use crate::models::feed::{CreateFeed, Feed};
use crate::partials::add_feed_form::add_feed_form;
use crate::partials::entry_link::entry_link;
use crate::partials::{entry_list::entry_list, feed_link::feed_link, layout::Layout};
use crate::state::Crawls;
use crate::uuid::Base62Uuid;

pub async fn get(
    Path(id): Path<Base62Uuid>,
    State(pool): State<PgPool>,
    layout: Layout,
) -> Result<Response> {
    let feed = Feed::get(&pool, id.as_uuid()).await?;
    let options = GetEntriesOptions {
        feed_id: Some(feed.feed_id),
        ..Default::default()
    };
    let title = feed.title.unwrap_or_else(|| "Untitled Feed".to_string());
    let entries = Entry::get_all(&pool, &options).await?;
    let delete_url = format!("/feed/{}/delete", id);
    Ok(layout.with_subtitle(&title).render(html! {
        header class="feed-header" {
            h2 { (title) }
            button class="edit-feed" { "✏️ Edit feed" }
            form action=(delete_url) method="post" {
                button type="submit" class="remove-feed" data-controller="remove-feed" { "❌ Remove feed" }
            }
        }
        @if let Some(description) = feed.description {
            p { (description) }
        }
        (entry_list(entries, &options))
    }))
}

#[serde_as]
#[derive(Deserialize)]
pub struct AddFeed {
    url: String,
    #[serde_as(as = "NoneAsEmptyString")]
    title: Option<String>,
    #[serde_as(as = "NoneAsEmptyString")]
    description: Option<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum AddFeedError {
    #[error("invalid feed url: {0}")]
    InvalidUrl(String, #[source] url::ParseError),
    #[error("failed to fetch feed: {0}")]
    FetchError(String, #[source] reqwest::Error),
    #[error("failed to parse feed: {0}")]
    ParseError(String, #[source] parser::ParseFeedError),
    #[error("failed to create feed: {0}")]
    CreateFeedError(String, #[source] Error),
    #[error("feed already exists: {0}")]
    FeedAlreadyExists(String, #[source] Error),
}
pub type AddFeedResult<T, E = AddFeedError> = ::std::result::Result<T, E>;

impl AddFeedError {
    fn status_code(&self) -> StatusCode {
        use AddFeedError::*;

        match self {
            InvalidUrl(..) | FetchError(..) | ParseError(..) | FeedAlreadyExists(..) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            CreateFeedError(..) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AddFeedError {
    fn into_response(self) -> Response {
        (
            self.status_code(),
            html! {
                (add_feed_form())
                ul class="stream-messages" {
                    li { span class="error" { (self) } }
                }
            }
            .into_string(),
        )
            .into_response()
    }
}

pub async fn post(
    State(pool): State<PgPool>,
    State(crawls): State<Crawls>,
    State(crawl_scheduler): State<CrawlSchedulerHandle>,
    Form(add_feed): Form<AddFeed>,
) -> AddFeedResult<Response> {
    let feed = Feed::create(
        &pool,
        CreateFeed {
            title: add_feed.title,
            url: add_feed.url.clone(),
            description: add_feed.description,
        },
    )
    .await
    .map_err(|err| {
        if let Error::Sqlx(sqlx::error::Error::Database(db_error)) = &err {
            if let Some(code) = db_error.code() {
                if let Some(constraint) = db_error.constraint() {
                    if code == "23505" && constraint == "feed_url_idx" {
                        return AddFeedError::FeedAlreadyExists(add_feed.url.clone(), err);
                    }
                }
            }
        }
        AddFeedError::CreateFeedError(add_feed.url.clone(), err)
    })?;

    let receiver = crawl_scheduler.schedule(feed.feed_id).await;
    {
        let mut crawls = crawls.lock().await;
        crawls.insert(feed.feed_id, receiver);
    }

    let feed_stream = format!("connect:/feed/{}/stream", Base62Uuid::from(feed.feed_id));
    Ok((
        StatusCode::CREATED,
        html! {
            (add_feed_form())
            div hx-sse=(feed_stream) {
                ul class="stream-messages" hx-sse="swap:message" hx-swap="beforeend" {
                    li { "Fetching feed..." }
                }
            }
        }
        .into_string(),
    )
        .into_response())
}

pub async fn stream(
    Path(id): Path<Base62Uuid>,
    State(crawls): State<Crawls>,
) -> Result<impl IntoResponse> {
    let receiver = {
        let mut crawls = crawls.lock().await;
        crawls.remove(&id.as_uuid())
    }
    .ok_or_else(|| Error::NotFound("feed stream", id.as_uuid()))?;

    let stream = BroadcastStream::new(receiver);
    let feed_id = format!("feed-{}", id);
    let stream = stream.map(move |msg| match msg {
        Ok(CrawlSchedulerHandleMessage::FeedCrawler(FeedCrawlerHandleMessage::Feed(Ok(feed)))) => {
            Ok::<Event, String>(
                Event::default().data(
                    html! {
                        li { "Crawled feed: " (feed_link(&feed, false)) }
                    }
                    .into_string(),
                ),
            )
        }
        Ok(CrawlSchedulerHandleMessage::FeedCrawler(FeedCrawlerHandleMessage::Entry(Ok(
            entry,
        )))) => Ok(Event::default().data(
            html! {
                li { "Crawled entry: " (entry_link(&entry)) }
            }
            .into_string(),
        )),
        Ok(CrawlSchedulerHandleMessage::FeedCrawler(FeedCrawlerHandleMessage::Feed(Err(
            error,
        )))) => Ok(Event::default().data(
            html! {
                li id=(feed_id) { span class="error" { (error) } }
            }
            .into_string(),
        )),
        Ok(CrawlSchedulerHandleMessage::FeedCrawler(FeedCrawlerHandleMessage::Entry(Err(
            error,
        )))) => Ok(Event::default().data(
            html! {
                li { span class="error" { (error) } }
            }
            .into_string(),
        )),
        Ok(CrawlSchedulerHandleMessage::Schedule(Err(error))) => Ok(Event::default().data(
            html! {
                li { span class="error" { (error) } }
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

pub async fn delete(State(pool): State<PgPool>, Path(id): Path<Base62Uuid>) -> Result<Redirect> {
    Feed::delete(&pool, id.as_uuid()).await?;
    Ok(Redirect::to("/feeds"))
}
