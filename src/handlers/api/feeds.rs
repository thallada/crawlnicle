use axum::extract::Query;
use axum::extract::State;
use axum::response::IntoResponse;
use axum_extra::TypedHeader;
use sqlx::PgPool;

use crate::api_response::ApiResponse;
use crate::error::Error;
use crate::headers::Accept;
use crate::models::feed::{Feed, GetFeedsOptions};
use crate::partials::feed_list::feed_list;

pub async fn get(
    Query(options): Query<GetFeedsOptions>,
    accept: Option<TypedHeader<Accept>>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let feeds = Feed::get_all(&pool, &options).await.map_err(Error::from)?;
    if let Some(TypedHeader(accept)) = accept {
        if accept == Accept::ApplicationJson {
            return Ok::<ApiResponse<Vec<Feed>>, Error>(ApiResponse::Json(feeds));
        }
    }
    Ok(ApiResponse::Html(
        feed_list(feeds, &options, false).into_string(),
    ))
}
