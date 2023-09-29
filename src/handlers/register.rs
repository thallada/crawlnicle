use std::time::Duration;

use axum::response::{IntoResponse, Response};
use axum::TypedHeader;
use axum::{extract::State, Form};
use chrono::Utc;
use lettre::message::{Mailbox, Message, MultiPart};
use lettre::{SmtpTransport, Transport};
use maud::html;
use serde::Deserialize;
use serde_with::{serde_as, NoneAsEmptyString};
use sqlx::PgPool;
use tracing::error;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::htmx::{HXRedirect, HXTarget};
use crate::models::user::{AuthContext, CreateUser, User};
use crate::models::user_email_verification_token::{
    CreateUserEmailVerificationToken, UserEmailVerificationToken,
};
use crate::partials::layout::Layout;
use crate::partials::register_form::{register_form, RegisterFormProps};
use crate::uuid::Base62Uuid;

// TODO: put in config
const USER_EMAIL_VERIFICATION_TOKEN_EXPIRATION: Duration = Duration::from_secs(24 * 60 * 60);

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

pub fn send_confirmation_email(pool: PgPool, mailer: SmtpTransport, config: Config, user: User) {
    tokio::spawn(async move {
        let user_email_address = match user.email.parse() {
            Ok(address) => address,
            Err(err) => {
                error!("failed to parse email address: {}", err);
                return;
            }
        };
        let mailbox = Mailbox::new(user.name.clone(), user_email_address);
        let token = match UserEmailVerificationToken::create(
            &pool,
            CreateUserEmailVerificationToken {
                user_id: user.user_id,
                expires_at: Utc::now() + USER_EMAIL_VERIFICATION_TOKEN_EXPIRATION,
            },
        )
        .await
        {
            Ok(token) => token,
            Err(err) => {
                error!("failed to create user email verification token: {}", err);
                return;
            }
        };
        let confirm_link = format!(
            "{}/confirm-email?token_id={}",
            config.public_url,
            Base62Uuid::from(token.token_id)
        );
        let email = match Message::builder()
            .from(config.email_from.clone())
            .to(mailbox)
            .subject("Welcome to crawlnicle, please confirm your email address")
            .multipart(MultiPart::alternative_plain_html(
                format!("Welcome to crawlnicle!\n\nPlease confirm your email address\n\nClick here to confirm your email address: {}", confirm_link),
                html! {
                    h1 { "Welcome to crawlnicle!" }
                    h2 { "Please confirm your email address" }
                    p {
                        a href=(confirm_link) { "Click here to confirm your email address" }
                    }
                }.into_string(),
            ))
        {
            Ok(email) => email,
            Err(err) => {
                error!("failed to create email: {}", err);
                return;
            }
        };

        // TODO: notify the user that email has been sent somehow
        match mailer.send(&email) {
            Ok(_) => (),
            Err(err) => {
                error!("failed to send email: {}", err);
            }
        }
    });
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
