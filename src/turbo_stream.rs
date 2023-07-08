use axum::response::{IntoResponse, Response};
use axum::http::{header, HeaderValue};
use axum::body::{Bytes, Full};

/// A Turbo Stream HTML response.
///
/// See [the Turbo Streams specification](https://turbo.hotwire.dev/handbook/streams) for more 
/// details.
///
/// Will automatically get `Content-Type: text/vnd.turbo-stream.html`.
#[derive(Clone, Copy, Debug)]
pub struct TurboStream<T>(pub T);

impl<T> IntoResponse for TurboStream<T>
where
    T: Into<Full<Bytes>>,
{
    fn into_response(self) -> Response {
        (
            [(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/vnd.turbo-stream.html"),
            )],
            self.0.into(),
        )
            .into_response()
    }
}

impl<T> From<T> for TurboStream<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}
