use std::path::Path;
use std::fs;
#[cfg(not(debug_assertions))]
use std::str::Lines;

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, State},
    http::request::Parts,
    response::{Html, IntoResponse, Response},
};
use maud::{html, Markup, DOCTYPE};

#[cfg(not(debug_assertions))]
use crate::{JS_BUNDLES, CSS_BUNDLES};
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

// In development, the JS and CSS file names are retrieved at runtime during Layout::render so that 
// the server binary does not need to be rebuilt when frontend files are changed.
//
// In release mode, this work is done ahead of time in build.rs and saved to static/js/manifest.txt 
// and static/css/manifest.txt. The contents of those files are then compiled into the server 
// binary so that rendering the Layout does not need to do any filesystem operations.
fn get_bundles(asset_type: &str) -> Vec<String> {
    let root_dir = Path::new("./");
    let dir = root_dir.join(format!("static/{}", asset_type));

    let entries = fs::read_dir(dir).unwrap();

    entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .ends_with(&format!(".{}", asset_type))
        })
        .map(|entry| {
            Path::new("/")
                .join(entry.path().strip_prefix(root_dir).unwrap())
                .display()
                .to_string()
        }).collect()
}

#[cfg(debug_assertions)]
fn js_bundles() -> Vec<String> {
    get_bundles("js")
}

#[cfg(not(debug_assertions))]
fn js_bundles() -> Lines<'static> {
    JS_BUNDLES.lines()
}

#[cfg(debug_assertions)]
fn css_bundles() -> Vec<String> {
    get_bundles("css")
}

#[cfg(not(debug_assertions))]
fn css_bundles() -> Lines<'static> {
    CSS_BUNDLES.lines()
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
                    @for js_bundle in js_bundles() {
                        script type="module" src=(js_bundle) {}
                    }
                    @for css_bundle in css_bundles() {
                        link rel="stylesheet" href=(css_bundle) {}
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
