use axum::response::{IntoResponse, Response};
use axum::Form;
use axum_extra::TypedHeader;
use http::HeaderValue;
use maud::html;
use serde::Deserialize;
use serde_with::serde_as;
use tracing::info;

use crate::auth::{AuthSession, Credentials};
use crate::error::{Error, Result};
use crate::htmx::{HXRedirect, HXRequest, HXTarget};
use crate::partials::login_form::{login_form, LoginFormProps};
use crate::{models::user::User, partials::layout::Layout};

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct Login {
    email: String,
    password: String,
}

impl From<Login> for Credentials {
    fn from(login: Login) -> Self {
        Credentials {
            email: login.email,
            password: login.password,
        }
    }
}

pub fn login_page(
    hx_target: Option<TypedHeader<HXTarget>>,
    layout: Layout,
    form_props: LoginFormProps,
) -> Response {
    if let Some(hx_target) = &hx_target {
        if hx_target.target == HeaderValue::from_static("login-form") {
            return login_form(form_props).into_response();
        }
    }
    layout
        .with_subtitle("login")
        .targeted(hx_target)
        .render(html! {
            div class="center-horizontal" {
                header class="center-text" {
                    h2 { "Login" }
                }
                (login_form(form_props))
            }
        })
        .into_response()
}

pub async fn get(hx_target: Option<TypedHeader<HXTarget>>, layout: Layout) -> Result<Response> {
    Ok(login_page(hx_target, layout, LoginFormProps::default()))
}

pub async fn post(
    mut auth: AuthSession,
    hx_target: Option<TypedHeader<HXTarget>>,
    hx_request: Option<TypedHeader<HXRequest>>,
    layout: Layout,
    Form(login): Form<Login>,
) -> Result<Response> {
    let user: User = match auth.authenticate(login.clone().into()).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            info!(email = login.email, "authentication failed");
            return Ok(login_page(
                hx_target,
                layout,
                LoginFormProps {
                    email: Some(login.email),
                    general_error: Some("invalid email or password".to_string()),
                    ..Default::default()
                },
            ));
        }
        Err(_) => {
            return Err(Error::InternalServerError);
        }
    };
    info!(user_id = %user.user_id, "login successful");
    auth.login(&user)
        .await
        .map_err(|_| Error::InternalServerError)?;
    Ok(HXRedirect::to("/")
        .is_htmx(hx_request.is_some())
        .reload(true)
        .into_response())
}
