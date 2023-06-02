use axum::{
        async_trait,
        extract::FromRequestParts,
        http::request::Parts,
        response::{Html, IntoResponse, Response},
};
use maud::{DOCTYPE, html, Markup};

use crate::partials::header::header;

pub struct Layout;

#[async_trait]
impl<S> FromRequestParts<S> for Layout
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // extract whatever your layout needs
        Ok(Self {})
    }
}

impl Layout {
    pub fn render(self, template: Markup) -> Response {
        let with_layout = html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    meta charset="utf-8";
                    title { "crawlnicle" }
                    script type="module" {
                        r#"import * as Turbo from 'https://cdn.skypack.dev/@hotwired/turbo';"#
                    }
                }
                body {
                    (header())
                    (template)
                }
            }
        }.into_string();

        Html(with_layout).into_response()
    }
}
