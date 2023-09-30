use axum::response::{IntoResponse, Response};
use axum::TypedHeader;
use axum::{extract::State, Form};
use maud::html;
use serde::Deserialize;
use serde_with::serde_as;
use sqlx::PgPool;
use tracing::info;

use crate::auth::verify_password;
use crate::error::{Error, Result};
use crate::htmx::{HXRedirect, HXTarget};
use crate::partials::login_form::{login_form, LoginFormProps};
use crate::{
    models::user::{AuthContext, User},
    partials::layout::Layout,
};

#[serde_as]
#[derive(Deserialize)]
pub struct Login {
    email: String,
    password: String,
}

pub fn login_page(
    hx_target: Option<TypedHeader<HXTarget>>,
    layout: Layout,
    form_props: LoginFormProps,
) -> Response {
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
    State(pool): State<PgPool>,
    mut auth: AuthContext,
    hx_target: Option<TypedHeader<HXTarget>>,
    layout: Layout,
    Form(login): Form<Login>,
) -> Result<Response> {
    let user: User = match User::get_by_email(&pool, login.email.clone()).await {
        Ok(user) => user,
        Err(err) => {
            if let Error::NotFoundString(_, _) = err {
                info!(email = login.email, "invalid enail");
                return Ok(login_page(
                    hx_target,
                    layout,
                    LoginFormProps {
                        email: Some(login.email),
                        general_error: Some("invalid email or password".to_string()),
                        ..Default::default()
                    },
                ));
            } else {
                return Err(err);
            }
        }
    };
    if verify_password(login.password, user.password_hash.clone())
        .await
        .is_err()
    {
        info!(user_id = %user.user_id, "invalid password");
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
    info!(user_id = %user.user_id, "login successful");
    auth.login(&user)
        .await
        .map_err(|_| Error::InternalServerError)?;
    Ok(HXRedirect::to("/").into_response())
}
