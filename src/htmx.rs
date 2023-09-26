use axum::response::{IntoResponse, Response};
use http::header::{HeaderName, HeaderValue};
use http::StatusCode;

#[allow(clippy::declare_interior_mutable_const)]
const HX_LOCATION: HeaderName = HeaderName::from_static("hx-location");

/// Sets the HX-Location header so that HTMX redirects to the given URI. Unlike 
/// axum::response::Redirect this does not return a 300-level status code (instead, a 200 status 
/// code) so that HTMX can see the HX-Location header before the browser handles the redirect.
#[derive(Debug, Clone)]
pub struct HXRedirect {
    location: HeaderValue,
}

impl HXRedirect {
    pub fn to(uri: &str) -> Self {
        Self {
            location: HeaderValue::try_from(uri).expect("URI isn't a valid header value"),
        }
    }
}

impl IntoResponse for HXRedirect {
    fn into_response(self) -> Response {
        (
            StatusCode::OK,
            [(HX_LOCATION, self.location)],
        )
            .into_response()
    }
}
