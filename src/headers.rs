use axum::http::{HeaderName, HeaderValue};
use axum_extra::headers::{self, Header};

/// Typed header implementation for the `Accept` header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accept {
    TextHtml,
    ApplicationJson,
}

impl std::fmt::Display for Accept {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Accept::TextHtml => write!(f, "text/html"),
            Accept::ApplicationJson => write!(f, "application/json"),
        }
    }
}

impl Header for Accept {
    fn name() -> &'static HeaderName {
        &http::header::ACCEPT
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values.next().ok_or_else(headers::Error::invalid)?;

        match value.to_str().map_err(|_| headers::Error::invalid())? {
            "text/html" => Ok(Accept::TextHtml),
            "application/json" => Ok(Accept::ApplicationJson),
            _ => Err(headers::Error::invalid()),
        }
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        values.extend(std::iter::once(self.into()));
    }
}

impl From<Accept> for HeaderValue {
    fn from(value: Accept) -> Self {
        HeaderValue::from(&value)
    }
}

impl From<&Accept> for HeaderValue {
    fn from(value: &Accept) -> Self {
        HeaderValue::from_str(value.to_string().as_str()).unwrap()
    }
}
