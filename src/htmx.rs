use axum::response::{IntoResponse, Response};
use headers::{self, Header};
use http::header::{HeaderName, HeaderValue};
use http::StatusCode;

#[allow(clippy::declare_interior_mutable_const)]
pub const HX_LOCATION: HeaderName = HeaderName::from_static("hx-location");
#[allow(clippy::declare_interior_mutable_const)]
pub const HX_BOOSTED: HeaderName = HeaderName::from_static("hx-boosted");

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

#[derive(Debug)]
pub struct HXBoosted(bool);

impl HXBoosted {
    pub fn is_boosted(&self) -> bool {
        self.0
    }
}

impl Header for HXBoosted {
    fn name() -> &'static HeaderName {
        static HX_BOOSTED_STATIC: HeaderName = HX_BOOSTED;
        &HX_BOOSTED_STATIC
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values
            .next()
            .ok_or_else(headers::Error::invalid)?;

        if value == "true" {
            Ok(HXBoosted(true))
        } else if value == "false" {
            Ok(HXBoosted(false))
        } else {
            Err(headers::Error::invalid())
        }
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        let s = if self.0 {
            "true"
        } else {
            "false"
        };

        let value = HeaderValue::from_static(s);

        values.extend(std::iter::once(value));
    }
}
