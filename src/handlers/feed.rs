use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response, Redirect};
use axum::Form;
use feed_rs::parser;
use maud::html;
use reqwest::Client;
use serde::Deserialize;
use serde_with::{serde_as, NoneAsEmptyString};
use sqlx::PgPool;

use crate::error::{Error, Result};
use crate::models::entry::get_entries_for_feed;
use crate::models::feed::{create_feed, get_feed, CreateFeed, delete_feed};
use crate::partials::{entry_list::entry_list, feed_link::feed_link, layout::Layout};
use crate::uuid::Base62Uuid;
use crate::turbo_stream::TurboStream;

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
    #[error("failed to fetch feed: {0}")]
    FetchError(String, #[source] reqwest::Error),
    #[error("failed to parse feed: {0}")]
    ParseError(String, #[source] parser::ParseFeedError),
    #[error("failed to create feed: {0}")]
    CreateFeedError(String, #[source] Error),
}
pub type AddFeedResult<T, E = AddFeedError> = ::std::result::Result<T, E>;

impl AddFeedError {
    fn status_code(&self) -> StatusCode {
        use AddFeedError::*;

        match self {
            FetchError(..) | ParseError(..) => StatusCode::UNPROCESSABLE_ENTITY,
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
    Form(add_feed): Form<AddFeed>,
) -> AddFeedResult<Response> {
    let client = Client::new();
    let bytes = client
        .get(&add_feed.url)
        .send()
        .await
        .map_err(|err| AddFeedError::FetchError(add_feed.url.clone(), err))?
        .bytes()
        .await
        .map_err(|err| AddFeedError::FetchError(add_feed.url.clone(), err))?;
    let parsed_feed = parser::parse(&bytes[..])
        .map_err(|err| AddFeedError::ParseError(add_feed.url.clone(), err))?;
    let feed = create_feed(
        &pool,
        CreateFeed {
            title: add_feed
                .title
                .map_or_else(|| parsed_feed.title.map(|text| text.content), Some),
            url: add_feed.url.clone(),
            feed_type: parsed_feed.feed_type.into(),
            description: add_feed
                .description
                .map_or_else(|| parsed_feed.description.map(|text| text.content), Some),
        },
    )
    .await
    .map_err(|err| AddFeedError::CreateFeedError(add_feed.url.clone(), err))?;
    Ok((
        StatusCode::CREATED,
        TurboStream(
            html! {
                turbo-stream action="append" target="feeds" {
                    template {
                        li { (feed_link(&feed)) }
                    }
                }
            }
            .into_string(),
        ),
    )
        .into_response())
}

pub async fn delete(
    State(pool): State<PgPool>,
    Path(id): Path<Base62Uuid>,
) -> Result<Redirect> {
    delete_feed(&pool, id.as_uuid()).await?;
    Ok(Redirect::to("/feeds"))
}
