use axum::response::{IntoResponse, Response};
use axum::TypedHeader;
use axum::{extract::State, Form};
use lettre::SmtpTransport;
use maud::html;
use serde::Deserialize;
use serde_with::{serde_as, NoneAsEmptyString};
use sqlx::PgPool;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::htmx::{HXRedirect, HXTarget};
use crate::mailers::email_verification::send_confirmation_email;
use crate::models::user::{AuthContext, CreateUser, User};
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

pub async fn get(hx_target: Option<TypedHeader<HXTarget>>, layout: Layout) -> Result<Response> {
    Ok(layout
        .with_subtitle("register")
        .targeted(hx_target)
        .render(html! {
            div class="center-horizontal" {
                header class="center-text" {
                    h2 { "Register" }
                }
                (register_form(RegisterFormProps::default()))
            }
        }))
}

pub async fn post(
    State(pool): State<PgPool>,
    State(mailer): State<SmtpTransport>,
    State(config): State<Config>,
    mut auth: AuthContext,
    Form(register): Form<Register>,
) -> Result<Response> {
    if register.password != register.password_confirmation {
        // return Err(Error::BadRequest("passwords do not match"));
        return Ok(register_form(RegisterFormProps {
            email: Some(register.email),
            name: register.name,
            password_error: Some("passwords do not match".to_string()),
            ..Default::default()
        })
        .into_response());
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
                return Ok(register_form(RegisterFormProps {
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
                })
                .into_response());
            }
            if let Error::Sqlx(sqlx::error::Error::Database(db_error)) = &err {
                if let Some(constraint) = db_error.constraint() {
                    if constraint == "users_email_idx" {
                        return Ok(register_form(RegisterFormProps {
                            email: Some(register.email),
                            name: register.name,
                            email_error: Some("email already exists".to_string()),
                            ..Default::default()
                        })
                        .into_response());
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
    Ok(HXRedirect::to("/").into_response())
}
