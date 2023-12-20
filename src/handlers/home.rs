use axum::extract::State;
use axum::response::Response;
use axum_extra::TypedHeader;
use maud::html;
use sqlx::PgPool;

use crate::error::Result;
use crate::htmx::HXTarget;
use crate::models::entry::Entry;
use crate::partials::{entry_list::entry_list, layout::Layout};

pub async fn get(
    State(pool): State<PgPool>,
    hx_target: Option<TypedHeader<HXTarget>>,
    layout: Layout,
) -> Result<Response> {
    let options = Default::default();
    let entries = Entry::get_all(&pool, &options).await?;
    Ok(layout.targeted(hx_target).render(html! {
        ul class="entries" {
            (entry_list(entries, &options))
        }
    }))
}
