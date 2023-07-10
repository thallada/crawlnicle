use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive};
use axum::response::{IntoResponse, Redirect, Response, Sse};
use axum::Form;
use feed_rs::parser;
use maud::html;
use reqwest::Client;
use serde::Deserialize;
use serde_with::{serde_as, NoneAsEmptyString};
use sqlx::PgPool;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use url::Url;

use crate::actors::feed_crawler::{FeedCrawlerHandle, FeedCrawlerHandleMessage};
use crate::error::{Error, Result};
use crate::models::entry::get_entries_for_feed;
use crate::models::feed::{create_feed, delete_feed, get_feed, CreateFeed, FeedType};
use crate::partials::{entry_list::entry_list, feed_link::feed_link, layout::Layout};
use crate::state::Crawls;
use crate::turbo_stream::TurboStream;
use crate::uuid::Base62Uuid;

pub async fn get(
    Path(id): Path<Base62Uuid>,
    State(pool): State<PgPool>,
    layout: Layout,
) -> Result<Response> {
    let feed = get_feed(&pool, id.as_uuid()).await?;
    let entries = get_entries_for_feed(&pool, feed.feed_id, Default::default()).await?;
    let delete_url = format!("/feed/{}/delete", id);
    Ok(layout.render(html! {
        header class="feed-header" {
            h2 { (feed.title.unwrap_or_else(|| "Untitled Feed".to_string())) }
            button class="edit-feed" { "✏️ Edit feed" }
            form action=(delete_url) method="post" {
                button type="submit" class="remove-feed" data-controller="remove-feed" { "❌ Remove feed" }
            }
        }
        @if let Some(description) = feed.description {
            p { (description) }
        }
        (entry_list(entries))
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
            TurboStream(
                html! {
                    turbo-stream action="append" target="feeds" {
                        template {
                            li { span class="error" { (self) } }
                        }
                    }
                }
                .into_string(),
            ),
        )
            .into_response()
    }
}

pub async fn post(
    State(pool): State<PgPool>,
    State(crawls): State<Crawls>,
    Form(add_feed): Form<AddFeed>,
) -> AddFeedResult<Response> {
    // TODO: store the client in axum state (as long as it can be used concurrently?)
    let client = Client::new();
    let feed_crawler = FeedCrawlerHandle::new(pool.clone(), client.clone());

    let feed = create_feed(
        &pool,
        CreateFeed {
            title: add_feed.title,
            url: add_feed.url.clone(),
            feed_type: FeedType::Rss, // eh, get rid of this
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

    let url: Url = Url::parse(&add_feed.url)
        .map_err(|err| AddFeedError::InvalidUrl(add_feed.url.clone(), err))?;
    let receiver = feed_crawler.crawl(url).await;
    {
        let mut crawls = crawls.lock().map_err(|_| {
            AddFeedError::CreateFeedError(add_feed.url.clone(), Error::InternalServerError)
        })?;
        crawls.insert(feed.feed_id, receiver);
    }

    let feed_id = format!("feed-{}", Base62Uuid::from(feed.feed_id));
    let feed_stream = format!("/feed/{}/stream", Base62Uuid::from(feed.feed_id));
    Ok((
        StatusCode::CREATED,
        TurboStream(
            html! {
                turbo-stream-source src=(feed_stream) id="feed-stream" {}
                turbo-stream action="append" target="feeds" {
                    template {
                        li id=(feed_id) { (feed_link(&feed, true)) }
                    }
                }
            }
            .into_string(),
        ),
    )
        .into_response())
}

pub async fn stream(
    Path(id): Path<Base62Uuid>,
    State(crawls): State<Crawls>,
) -> Result<impl IntoResponse> {
    let receiver = {
        let mut crawls = crawls.lock().expect("crawls lock poisoned");
        crawls.remove(&id.as_uuid())
    }
    .ok_or_else(|| Error::NotFound("feed stream", id.as_uuid()))?;

    let stream = BroadcastStream::new(receiver);
    let feed_id = format!("feed-{}", id);
    let stream = stream.map(move |msg| match msg {
        Ok(FeedCrawlerHandleMessage::Feed(Ok(feed))) => Ok::<Event, String>(
            Event::default().data(
                html! {
                    turbo-stream action="remove" target="feed-stream" {}
                    turbo-stream action="replace" target=(feed_id) {
                        template {
                            li id=(feed_id) { (feed_link(&feed, false)) }
                        }
                    }
                }
                .into_string(),
            ),
        ),
        Ok(FeedCrawlerHandleMessage::Feed(Err(error))) => Ok(Event::default().data(
            html! {
                turbo-stream action="remove" target="feed-stream" {}
                turbo-stream action="replace" target=(feed_id) {
                    template {
                        li id=(feed_id) { span class="error" { (error) } }
                    }
                }
            }
            .into_string(),
        )),
        // TODO: these Entry messages are not yet sent, need to handle them better
        Ok(FeedCrawlerHandleMessage::Entry(Ok(_))) => Ok(Event::default().data(
            html! {
                turbo-stream action="remove" target="feed-stream" {}
                turbo-stream action="replace" target=(feed_id) {
                    template {
                        li id=(feed_id) { "fetched entry" }
                    }
                }
            }
            .into_string(),
        )),
        Ok(FeedCrawlerHandleMessage::Entry(Err(error))) => Ok(Event::default().data(
            html! {
                turbo-stream action="remove" target="feed-stream" {}
                turbo-stream action="replace" target=(feed_id) {
                    template {
                        li id=(feed_id) { span class="error" { (error) } }
                    }
                }
            }
            .into_string(),
        )),
        Err(BroadcastStreamRecvError::Lagged(_)) => Ok(Event::default()),
    });
    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive-text"),
    ))
}

pub async fn delete(State(pool): State<PgPool>, Path(id): Path<Base62Uuid>) -> Result<Redirect> {
    delete_feed(&pool, id.as_uuid()).await?;
    Ok(Redirect::to("/feeds"))
}
