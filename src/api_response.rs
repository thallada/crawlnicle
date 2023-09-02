use axum::{
    response::{Html, IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// Wrapper type for API responses that allows endpoints to return either JSON or HTML in the same
/// route.
#[derive(Debug)]
pub enum ApiResponse<T> {
    Json(T),
    Html(String),
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        match self {
            ApiResponse::Json(json) => Json(json).into_response(),
            ApiResponse::Html(html) => Html(html).into_response(),
        }
    }
}
