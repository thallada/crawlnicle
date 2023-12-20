use axum::extract::Query;
use axum::extract::State;
use axum::response::IntoResponse;
use axum_extra::TypedHeader;
use sqlx::PgPool;

use crate::api_response::ApiResponse;
use crate::error::Error;
use crate::headers::Accept;
use crate::models::entry::{Entry, GetEntriesOptions};
use crate::partials::entry_list::entry_list;

pub async fn get(
    Query(options): Query<GetEntriesOptions>,
    accept: Option<TypedHeader<Accept>>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let entries = Entry::get_all(&pool, &options).await.map_err(Error::from)?;
    if let Some(TypedHeader(accept)) = accept {
        if accept == Accept::ApplicationJson {
            return Ok::<ApiResponse<Vec<Entry>>, Error>(ApiResponse::Json(entries));
        }
    }
    Ok(ApiResponse::Html(
        entry_list(entries, &options).into_string(),
    ))
}
