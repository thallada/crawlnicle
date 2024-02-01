use std::fmt;

use chrono::{DateTime, Utc};
use maud::{html, Markup};

use crate::utils::FormattedUtcTimestamp;

#[derive(Debug)]
pub enum LocalTimeType {
    Date,
    Relative,
}

impl fmt::Display for LocalTimeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LocalTimeType::Date => write!(f, "date"),
            LocalTimeType::Relative => write!(f, "relative"),
        }
    }
}

#[derive(Debug, Default)]
pub struct TimeProps {
    pub timestamp: DateTime<Utc>,
    pub local_time_type: Option<LocalTimeType>,
}

pub fn time(
    TimeProps {
        timestamp,
        local_time_type,
    }: TimeProps,
) -> Markup {
    let time = FormattedUtcTimestamp::from(timestamp);
    html! {
        time
            datetime=(time.rfc3339)
            data-local-time=(local_time_type.unwrap_or(LocalTimeType::Date))
            title=(time.human_readable)
        {
            (time.rfc3339)
        }
    }
}

pub fn date_time(timestamp: DateTime<Utc>) -> Markup {
    time(TimeProps {
        timestamp,
        local_time_type: Some(LocalTimeType::Date),
    })
}

pub fn relative_time(timestamp: DateTime<Utc>) -> Markup {
    time(TimeProps {
        timestamp,
        local_time_type: Some(LocalTimeType::Relative),
    })
}
