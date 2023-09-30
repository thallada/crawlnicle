use axum::extract::{Query, State};
use axum::response::Response;
use axum::{TypedHeader, Form};
use lettre::SmtpTransport;
use maud::html;
use serde::Deserialize;
use serde_with::{serde_as, NoneAsEmptyString};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::htmx::HXTarget;
use crate::mailers::email_verification::send_confirmation_email;
use crate::models::user::User;
use crate::models::user_email_verification_token::UserEmailVerificationToken;
use crate::partials::layout::Layout;
use crate::partials::confirm_email_form::confirm_email_form;
use crate::uuid::Base62Uuid;

#[derive(Deserialize)]
pub struct ConfirmEmailQuery {
    pub token_id: Base62Uuid,
}

pub async fn get(
    State(pool): State<PgPool>,
    hx_target: Option<TypedHeader<HXTarget>>,
    layout: Layout,
    query: Query<ConfirmEmailQuery>,
) -> Result<Response> {
    let token = match UserEmailVerificationToken::get(&pool, query.token_id.as_uuid()).await {
        Ok(token) => token,
        Err(err) => {
            if let Error::NotFoundUuid(_, _) = err {
                return Ok(layout
                    .with_subtitle("confirm email")
                    .targeted(hx_target)
                    .render(html! {
                        div class="center-horizontal" {
                            header class="center-text" {
                                h2 { "Email verification token not found" }
                                p { "Enter your email to resend the confirmation email. If you don't have an account yet, create one " a href="/register" { "here" } "." }
                                (confirm_email_form(None))
                            }
                        }
                    }))
            }
            return Err(err);
        }
    };
    if token.expired() {
        Ok(layout
            .with_subtitle("confirm email")
            .targeted(hx_target)
            .render(html! {
                div class="center-horizontal" {
                    header class="center-text" {
                        h2 { "Email verification token is expired" }
                        p { "Click the button below to resend a new confirmation email. The email will be valid for another 24 hours."}
                        (confirm_email_form(Some(token)))
                    }
                }
            }))
    } else {
        User::verify_email(&pool, token.user_id).await?;
        UserEmailVerificationToken::delete(&pool, token.token_id).await?;
        Ok(layout
            .with_subtitle("confirm email")
            .targeted(hx_target)
            .render(html! {
                div class="center-horizontal" {
                    header class="center-text" {
                        h2 { "Your email is now confirmed!" }
                        p {
                            "Thanks for verifying your email address. "
                            a href="/" { "Return home" }
                        }
                    }
                }
            }))
    }
}

#[serde_as]
#[derive(Deserialize)]
pub struct ConfirmEmail {
    #[serde_as(as = "NoneAsEmptyString")]
    token: Option<Uuid>,
    #[serde_as(as = "NoneAsEmptyString")]
    email: Option<String>,
}

pub async fn post(
    State(pool): State<PgPool>,
    State(mailer): State<SmtpTransport>,
    State(config): State<Config>,
    hx_target: Option<TypedHeader<HXTarget>>,
    layout: Layout,
    Form(confirm_email): Form<ConfirmEmail>,
) -> Result<Response> {
    if let Some(token_id) = confirm_email.token {
        let token = UserEmailVerificationToken::get(&pool, token_id).await?;
        let user = User::get(&pool, token.user_id).await?;
        if !user.email_verified {
            send_confirmation_email(pool, mailer, config, user);
        }
        return Ok(layout
            .with_subtitle("confirm email")
            .targeted(hx_target)
            .render(html! {
                div class="center-horizontal" {
                    header class="center-text" {
                        h2 { "Resent confirmation email" }
                        p {
                            "Please follow the link sent in the email."
                        }
                    }
                }
            }));
    }
    if let Some(email) = confirm_email.email {
        if let Ok(user) = User::get_by_email(&pool, email).await {
            if !user.email_verified {
                send_confirmation_email(pool, mailer, config, user);
            }
        }
        return Ok(layout
            .with_subtitle("confirm email")
            .targeted(hx_target)
            .render(html! {
                div class="center-horizontal" {
                    header class="center-text" {
                        h2 { "Resent confirmation email" }
                        p {
                            "If the email you entered matched an existing account, then a confirmation email was sent. Please follow the link sent in the email."
                        }
                    }
                }
            }));
    }
    Ok(layout
        .with_subtitle("confirm email")
        .targeted(hx_target)
        .render(html! {
            div class="center-horizontal" {
                header class="center-text" {
                    h2 { "Email verification token not found" }
                    p { "Enter your email to resend the confirmation email. If you don't have an account yet, create one " a href="/register" { "here" } "." }
                    (confirm_email_form(None))
                }
            }
        }))
}
