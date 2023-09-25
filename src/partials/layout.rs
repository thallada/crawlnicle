use std::fs;
use std::path::Path;
#[cfg(not(debug_assertions))]
use std::str::Lines;

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, State},
    http::request::Parts,
    response::{Html, IntoResponse, Response},
};
use axum_login::{extractors::AuthContext, SqlxStore};
use maud::{html, Markup, DOCTYPE};
use sqlx::PgPool;
use uuid::Uuid;

use crate::partials::header::header;
use crate::{config::Config, partials::footer::footer};
use crate::models::user::User;
#[cfg(not(debug_assertions))]
use crate::{CSS_MANIFEST, JS_MANIFEST};

#[derive(Debug, Default)]
pub struct Layout {
    pub title: String,
    pub subtitle: Option<String>,
    pub user: Option<User>,
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
        let auth_context = AuthContext::<Uuid, User, SqlxStore<PgPool, User>>::from_request_parts(parts, state)
            .await
            .map_err(|err| err.into_response())?;
        Ok(Self {
            title: config.title,
            user: auth_context.current_user,
            ..Default::default()
        })
    }
}

// In development, the JS and CSS file names are retrieved at runtime during Layout::render so that
// the server binary does not need to be rebuilt when frontend files are changed.
//
// In release mode, this work is done ahead of time in build.rs and saved to static/js/manifest.txt
// and static/css/manifest.txt. The contents of those files are then compiled into the server
// binary so that rendering the Layout does not need to do any filesystem operations.
fn get_manifest(asset_type: &str) -> Vec<String> {
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
        })
        .collect()
}

#[cfg(debug_assertions)]
fn js_manifest() -> Vec<String> {
    get_manifest("js")
}

#[cfg(not(debug_assertions))]
fn js_manifest() -> Lines<'static> {
    JS_MANIFEST.lines()
}

#[cfg(debug_assertions)]
fn css_manifest() -> Vec<String> {
    get_manifest("css")
}

#[cfg(not(debug_assertions))]
fn css_manifest() -> Lines<'static> {
    CSS_MANIFEST.lines()
}

impl Layout {
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn with_subtitle(mut self, subtitle: &str) -> Self {
        self.subtitle = Some(subtitle.to_string());
        self
    }

    pub fn with_user(mut self, user: User) -> Self {
        self.user = Some(user);
        self
    }

    fn full_title(&self) -> String {
        if let Some(subtitle) = &self.subtitle {
            format!("{} - {}", self.title, subtitle)
        } else {
            self.title.to_string()
        }
    }

    pub fn render(self, template: Markup) -> Response {
        let with_layout = html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    meta charset="utf-8";
                    title { (self.full_title()) }
                    @for js_file in js_manifest() {
                        script type="module" src=(js_file) {}
                    }
                    @for css_file in css_manifest() {
                        link rel="stylesheet" href=(css_file) {}
                    }
                }
                body hx-booster="true" {
                    (header(&self.title, self.user))
                    (template)
                    (footer())
                }
            }
        }
        .into_string();

        Html(with_layout).into_response()
    }
}
