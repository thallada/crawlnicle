use std::fs;
use std::path::Path;
#[cfg(not(debug_assertions))]
use std::str::Lines;

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, State},
    http::request::Parts,
    response::{IntoResponse, Response},
    TypedHeader,
};
use axum_login::{extractors::AuthContext, SqlxStore};
use headers::HeaderValue;
use maud::{html, Markup, DOCTYPE};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::user::User;
use crate::config::Config;
use crate::htmx::HXTarget;
use crate::partials::header::header;
use crate::partials::footer::footer;
#[cfg(not(debug_assertions))]
use crate::{CSS_MANIFEST, JS_MANIFEST};

#[derive(Debug, Default)]
pub struct Layout {
    pub title: String,
    pub subtitle: Option<String>,
    pub user: Option<User>,
    pub main_content_targeted: bool,
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
        let auth_context =
            AuthContext::<Uuid, User, SqlxStore<PgPool, User>>::from_request_parts(parts, state)
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

    /// If the given HX-Target is present and equal to "main-content", then this function will make 
    /// this Layout skip rendering the layout and only render the template with a hx-swap-oob 
    /// <title> element to update the document title.
    ///
    /// Links and forms that are boosted with the hx-boost attribute are only updating a portion of 
    /// the page inside the layout, so there is no need to render and send the layout again.
    pub fn targeted(mut self, hx_target: Option<TypedHeader<HXTarget>>) -> Self {
        if let Some(hx_target) = hx_target {
            if hx_target.target == HeaderValue::from_static("main-content") {
                self.main_content_targeted = true;
            }
        }
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
        if self.main_content_targeted {
            html! {
                title hx-swap-oob="true" { (self.full_title()) }
                (template)
            }
            .into_response()
        } else {
            html! {
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
                    body hx-boost="true" hx-target="#main-content" {
                        (header(&self.title, self.user))
                        main id="main-content" { (template) }
                        (footer())
                    }
                }
            }
            .into_response()
        }
    }
}
