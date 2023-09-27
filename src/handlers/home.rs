use axum::extract::State;
use axum::response::Response;
use axum::TypedHeader;
use maud::html;
use sqlx::PgPool;

use crate::error::Result;
use crate::htmx::HXBoosted;
use crate::models::entry::Entry;
use crate::partials::{entry_list::entry_list, layout::Layout};

pub async fn get(
    State(pool): State<PgPool>,
    hx_boosted: Option<TypedHeader<HXBoosted>>,
    layout: Layout,
) -> Result<Response> {
    let options = Default::default();
    let entries = Entry::get_all(&pool, &options).await?;
    Ok(layout.boosted(hx_boosted).render(html! {
        ul class="entries" {
            (entry_list(entries, &options))
        }
    }))
}
