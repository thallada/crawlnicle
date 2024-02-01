use chrono::{DateTime, SecondsFormat, Utc};
use url::Url;

pub fn get_domain(url: &str) -> Option<String> {
    Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(|s| s.to_string()))
        .map(|domain| {
            if domain.starts_with("www.") && domain.matches('.').count() > 1 {
                domain[4..].to_string()
            } else {
                domain
            }
        })
}

pub struct FormattedUtcTimestamp {
    pub rfc3339: String,
    pub human_readable: String,
}

impl From<DateTime<Utc>> for FormattedUtcTimestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        FormattedUtcTimestamp {
            rfc3339: dt.to_rfc3339_opts(SecondsFormat::Millis, true),
            human_readable: dt.format("%Y-%m-%d %H:%M:%S %Z").to_string(),
        }
    }
}
