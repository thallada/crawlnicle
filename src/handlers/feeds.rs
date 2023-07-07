use axum::extract::State;
use axum::response::Response;
use maud::html;
use sqlx::PgPool;

use crate::error::Result;
use crate::models::feed::get_feeds;
use crate::partials::layout::Layout;
use crate::uuid::Base62Uuid;

pub async fn get(State(pool): State<PgPool>, layout: Layout) -> Result<Response> {
    let feeds = get_feeds(&pool).await?;
    Ok(layout.render(html! {
        ul {
            @for feed in feeds {
                @let title = feed.title.unwrap_or_else(|| "Untitled Feed".to_string());
                @let feed_url = format!("/feed/{}", Base62Uuid::from(feed.feed_id));
                li { a href=(feed_url) { (title) } }
            }
        }
    }))
}
