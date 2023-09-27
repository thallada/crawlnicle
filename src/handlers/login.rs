use axum::response::{IntoResponse, Response};
use axum::TypedHeader;
use axum::{extract::State, Form};
use maud::html;
use serde::Deserialize;
use serde_with::serde_as;
use sqlx::PgPool;

use crate::auth::verify_password;
use crate::error::{Error, Result};
use crate::htmx::{HXBoosted, HXRedirect};
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

pub async fn get(hx_boosted: Option<TypedHeader<HXBoosted>>, layout: Layout) -> Result<Response> {
    Ok(layout
        .with_subtitle("login")
        .boosted(hx_boosted)
        .render(html! {
            div class="center-horizontal" {
                header class="center-text" {
                    h2 { "Login" }
                }
                (login_form(LoginFormProps::default()))
            }
        }))
}

pub async fn post(
    State(pool): State<PgPool>,
    mut auth: AuthContext,
    Form(login): Form<Login>,
) -> Result<Response> {
    let user: User = match User::get_by_email(&pool, login.email.clone()).await {
        Ok(user) => user,
        Err(err) => {
            if let Error::NotFoundString(_, _) = err {
                // Error::BadRequest("invalid email or password")
                return Ok(login_form(LoginFormProps {
                    email: Some(login.email),
                    general_error: Some("invalid email or password".to_string()),
                    ..Default::default()
                })
                .into_response());
            } else {
                return Err(err);
            }
        }
    };
    if verify_password(login.password, user.password_hash.clone())
        .await
        .is_err()
    {
        // return Err(Error::BadRequest("invalid email or password"));
        return Ok(login_form(LoginFormProps {
            email: Some(login.email),
            general_error: Some("invalid email or password".to_string()),
            ..Default::default()
        })
        .into_response());
    }
    auth.login(&user)
        .await
        .map_err(|_| Error::InternalServerError)?;
    Ok(HXRedirect::to("/").into_response())
}
