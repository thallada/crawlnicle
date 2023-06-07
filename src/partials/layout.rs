use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, State},
    http::request::Parts,
    response::{Html, IntoResponse, Response},
};
use maud::{html, Markup, DOCTYPE};

use crate::config::Config;
use crate::partials::header::header;

pub struct Layout {
    pub title: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for Layout
where
    S: Send + Sync,
    Config: FromRef<S>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let State(config) = State::<Config>::from_request_parts(parts, state)
            .await
            .map_err(|err| err.into_response())?;
        Ok(Self {
            title: config.title,
        })
    }
}

impl Layout {
    pub fn render(self, template: Markup) -> Response {
        let with_layout = html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    meta charset="utf-8";
                    title { (self.title) }
                    script type="module" {
                        r#"import * as Turbo from 'https://cdn.skypack.dev/@hotwired/turbo';"#
                    }
                }
                body {
                    (header(&self.title))
                    turbo-frame id="main" data-turbo-action="advance" {
                        (template)
                    }
                }
            }
        }
        .into_string();

        Html(with_layout).into_response()
    }
}
