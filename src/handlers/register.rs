use axum::response::{IntoResponse, Response};
use axum::{extract::State, Form};
use axum_extra::TypedHeader;
use http::HeaderValue;
use lettre::SmtpTransport;
use maud::html;
use serde::Deserialize;
use serde_with::{serde_as, NoneAsEmptyString};
use sqlx::PgPool;

use crate::auth::AuthSession;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::htmx::{HXRedirect, HXTarget};
use crate::mailers::email_verification::send_confirmation_email;
use crate::models::user::{CreateUser, User};
use crate::partials::layout::Layout;
use crate::partials::register_form::{register_form, RegisterFormProps};

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Register {
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
    #[serde_as(as = "NoneAsEmptyString")]
    pub name: Option<String>,
}

pub fn register_page(
    hx_target: Option<TypedHeader<HXTarget>>,
    layout: Layout,
    form_props: RegisterFormProps,
) -> Response {
    if let Some(hx_target) = &hx_target {
        if hx_target.target == HeaderValue::from_static("register-form") {
            return register_form(form_props).into_response();
        }
    }
    layout
        .with_subtitle("register")
        .targeted(hx_target)
        .render(html! {
            div class="center-horizontal" {
                header class="center-text" {
                    h2 { "Register" }
                }
                (register_form(form_props))
            }
        })
}

pub async fn get(hx_target: Option<TypedHeader<HXTarget>>, layout: Layout) -> Result<Response> {
    Ok(register_page(
        hx_target,
        layout,
        RegisterFormProps::default(),
    ))
}

pub async fn post(
    State(pool): State<PgPool>,
    State(mailer): State<SmtpTransport>,
    State(config): State<Config>,
    mut auth: AuthSession,
    hx_target: Option<TypedHeader<HXTarget>>,
    layout: Layout,
    Form(register): Form<Register>,
) -> Result<Response> {
    if register.password != register.password_confirmation {
        return Ok(register_page(
            hx_target,
            layout,
            RegisterFormProps {
                email: Some(register.email),
                name: register.name,
                password_error: Some("passwords do not match".to_string()),
                ..Default::default()
            },
        ));
    }
    let user = match User::create(
        &pool,
        CreateUser {
            email: register.email.clone(),
            password: register.password.clone(),
            name: register.name.clone(),
        },
    )
    .await
    {
        Ok(user) => user,
        Err(err) => {
            if let Error::InvalidEntity(validation_errors) = err {
                let field_errors = validation_errors.field_errors();
                return Ok(register_page(
                    hx_target,
                    layout,
                    RegisterFormProps {
                        email: Some(register.email),
                        name: register.name,
                        email_error: field_errors.get("email").map(|&errors| {
                            errors
                                .iter()
                                .filter_map(|error| error.message.clone().map(|m| m.to_string()))
                                .collect::<Vec<String>>()
                                .join(", ")
                        }),
                        name_error: field_errors.get("name").map(|&errors| {
                            errors
                                .iter()
                                .filter_map(|error| error.message.clone().map(|m| m.to_string()))
                                .collect::<Vec<String>>()
                                .join(", ")
                        }),
                        password_error: field_errors.get("password").map(|&errors| {
                            errors
                                .iter()
                                .filter_map(|error| error.message.clone().map(|m| m.to_string()))
                                .collect::<Vec<String>>()
                                .join(", ")
                        }),
                        ..Default::default()
                    },
                ));
            }
            if let Error::Sqlx(sqlx::error::Error::Database(db_error)) = &err {
                if let Some(constraint) = db_error.constraint() {
                    if constraint == "users_email_idx" {
                        return Ok(register_page(
                            hx_target,
                            layout,
                            RegisterFormProps {
                                email: Some(register.email),
                                name: register.name,
                                email_error: Some("email already exists".to_string()),
                                ..Default::default()
                            },
                        ));
                    }
                }
            }
            return Err(err);
        }
    };

    send_confirmation_email(pool, mailer, config, user.clone());

    auth.login(&user)
        .await
        .map_err(|_| Error::InternalServerError)?;
    Ok(HXRedirect::to("/").reload(true).into_response())
}
