use axum::extract::Request;
use axum::response::{IntoResponse, Redirect, Response};
use headers::{self, Header};
use http::header::{HeaderName, HeaderValue};
use http::StatusCode;

#[allow(clippy::declare_interior_mutable_const)]
pub const HX_REDIRECT: HeaderName = HeaderName::from_static("hx-redirect");
#[allow(clippy::declare_interior_mutable_const)]
pub const HX_LOCATION: HeaderName = HeaderName::from_static("hx-location");
#[allow(clippy::declare_interior_mutable_const)]
pub const HX_REQUEST: HeaderName = HeaderName::from_static("hx-request");
#[allow(clippy::declare_interior_mutable_const)]
pub const HX_TARGET: HeaderName = HeaderName::from_static("hx-target");

/// Sets the HX-Location header so that HTMX redirects to the given URI. Unlike
/// axum::response::Redirect this does not return a 300-level status code (instead, a 200 status
/// code) so that HTMX can see the HX-Location header before the browser handles the redirect.
#[derive(Debug, Clone)]
pub struct HXRedirect {
    location: HeaderValue,
    reload: bool,
    is_htmx_request: bool,
}

impl HXRedirect {
    pub fn to(uri: &str) -> Self {
        Self {
            location: HeaderValue::try_from(uri).expect("URI isn't a valid header value"),
            reload: false,
            is_htmx_request: true,
        }
    }

    pub fn reload(mut self, reload: bool) -> Self {
        self.reload = reload;
        self
    }

    pub fn is_htmx(mut self, is_htmx: bool) -> Self {
        self.is_htmx_request = is_htmx;
        self
    }
}

impl IntoResponse for HXRedirect {
    fn into_response(self) -> Response {
        if self.is_htmx_request {
            if self.reload {
                (StatusCode::OK, [(HX_REDIRECT, self.location)]).into_response()
            } else {
                (StatusCode::OK, [(HX_LOCATION, self.location)]).into_response()
            }
        } else {
            Redirect::to(self.location.to_str().unwrap()).into_response()
        }
    }
}

#[derive(Debug)]
pub struct HXRequest(bool);

impl HXRequest {
    pub fn is_true(&self) -> bool {
        self.0
    }
}

impl Header for HXRequest {
    fn name() -> &'static HeaderName {
        static HX_REQUEST_STATIC: HeaderName = HX_REQUEST;
        &HX_REQUEST_STATIC
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values.next().ok_or_else(headers::Error::invalid)?;

        if value == "true" {
            Ok(HXRequest(true))
        } else if value == "false" {
            Ok(HXRequest(false))
        } else {
            Err(headers::Error::invalid())
        }
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        let s = if self.0 { "true" } else { "false" };

        let value = HeaderValue::from_static(s);

        values.extend(std::iter::once(value));
    }
}

#[derive(Debug)]
pub struct HXTarget {
    pub target: HeaderValue,
}

impl Header for HXTarget {
    fn name() -> &'static HeaderName {
        static HX_TARGET_STATIC: HeaderName = HX_TARGET;
        &HX_TARGET_STATIC
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values.next().ok_or_else(headers::Error::invalid)?;

        Ok(HXTarget {
            target: value.clone(),
        })
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        values.extend(std::iter::once(self.target.clone()));
    }
}

pub fn not_htmx_predicate<Body>(req: &Request<Body>) -> bool {
    !req.headers().contains_key(HX_REQUEST)
}
