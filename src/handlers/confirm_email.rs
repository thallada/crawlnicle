use axum::extract::{Query, State};
use axum::response::Response;
use axum::{Form, TypedHeader};
use axum_login::SqlxStore;
use lettre::SmtpTransport;
use maud::html;
use serde::Deserialize;
use serde_with::{serde_as, NoneAsEmptyString};
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::htmx::HXTarget;
use crate::mailers::email_verification::send_confirmation_email;
use crate::models::user::{AuthContext, User};
use crate::models::user_email_verification_token::UserEmailVerificationToken;
use crate::partials::confirm_email_form::{confirm_email_form, ConfirmEmailFormProps};
use crate::partials::layout::Layout;
use crate::uuid::Base62Uuid;

#[derive(Deserialize)]
pub struct ConfirmEmailQuery {
    pub token_id: Option<Base62Uuid>,
}

pub async fn get(
    State(pool): State<PgPool>,
    auth: AuthContext,
    hx_target: Option<TypedHeader<HXTarget>>,
    layout: Layout,
    query: Query<ConfirmEmailQuery>,
) -> Result<Response> {
    if let Some(token_id) = query.token_id {
        info!(token_id = %token_id.as_uuid(), "get with token_id");
        let token = match UserEmailVerificationToken::get(&pool, token_id.as_uuid()).await {
            Ok(token) => token,
            Err(err) => {
                if let Error::NotFoundUuid(_, _) = err {
                    warn!(token_id = %token_id.as_uuid(), "token not found in database");
                    return Ok(layout
                        .with_subtitle("confirm email")
                        .targeted(hx_target)
                        .render(html! {
                            div class="center-horizontal" {
                                header class="center-text" {
                                    h2 { "Email verification token not found" }
                                }
                                p class="readable-width" { "Enter your email to resend the confirmation email. If you don't have an account yet, create one " a href="/register" { "here" } "." }
                                (confirm_email_form(ConfirmEmailFormProps::default()))
                            }
                        }));
                }
                return Err(err);
            }
        };
        if token.expired() {
            warn!(token_id = %token.token_id, "token expired");
            Ok(layout
                .with_subtitle("confirm email")
                .targeted(hx_target)
                .render(html! {
                    div class="center-horizontal" {
                        header class="center-text" {
                            h2 { "Email verification token is expired" }
                        }
                        p class="readable-width" { "Click the button below to resend a new confirmation email. The link in the email will be valid for another 24 hours."}
                        (confirm_email_form(ConfirmEmailFormProps { token: Some(token), email: None }))
                    }
                }))
        } else {
            info!(token_id = %token.token_id, "token valid, verifying email");
            User::verify_email(&pool, token.user_id).await?;
            UserEmailVerificationToken::delete(&pool, token.token_id).await?;
            Ok(layout
                .with_subtitle("confirm email")
                .targeted(hx_target)
                .render(html! {
                    div class="center-horizontal" {
                        header class="center-text" {
                            h2 { "Your email is now confirmed!" }
                        }
                        p class="readable-width" {
                            "Thanks for verifying your email address. "
                            a href="/" { "Return home" }
                        }
                    }
                }))
        }
    } else {
        Ok(layout
            .with_subtitle("confirm email")
            .targeted(hx_target)
            .render(html! {
                div class="center-horizontal" {
                    header class="center-text" {
                        h2 { "Confirm your email address" }
                    }
                    p class="readable-width" { "An email was sent to your email address upon registration containing a link that will confirm your email address. If you can't find it or it has been more than 24 hours since it was sent, you can resend the email by submitting the form below:"}
                    (confirm_email_form(
                        ConfirmEmailFormProps {
                            token: None,
                            email: auth.current_user.map(|u| u.email),
                        }
                    ))
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
        info!(%token_id, "posted with token_id");
        let token = UserEmailVerificationToken::get(&pool, token_id).await?;
        let user = User::get(&pool, token.user_id).await?;
        if !user.email_verified {
            info!(user_id = %user.user_id, "user exists, resending confirmation email");
            send_confirmation_email(pool, mailer, config, user);
        } else {
            warn!(user_id = %user.user_id, "confirm email submitted for already verified user, skip resend");
        }
        return Ok(layout
            .with_subtitle("confirm email")
            .targeted(hx_target)
            .render(html! {
                div class="center-horizontal" {
                    header class="center-text" {
                        h2 { "Resent confirmation email" }
                    }
                    p class="readable-width" {
                        "Please follow the link sent in the email."
                    }
                }
            }));
    }
    if let Some(email) = confirm_email.email {
        if let Ok(user) = User::get_by_email(&pool, email).await {
            if !user.email_verified {
                info!(user_id = %user.user_id, "user exists, resending confirmation email");
                send_confirmation_email(pool, mailer, config, user);
            } else {
                warn!(user_id = %user.user_id, "confirm email submitted for already verified user, skip resend");
            }
        }
        return Ok(layout
            .with_subtitle("confirm email")
            .targeted(hx_target)
            .render(html! {
                div class="center-horizontal" {
                    header class="center-text" {
                        h2 { "Resent confirmation email" }
                    }
                    p class="readable-width" {
                        "If the email you entered matched an existing account, then a confirmation email was sent. Please follow the link sent in the email."
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
                }
                p class="readable-width" { "Enter your email to resend the confirmation email." }
                p class="readable-width" {
                    "If you don't have an account yet, create one "
                    a href="/register" { "here" }
                    "."
                }
                (confirm_email_form(ConfirmEmailFormProps::default()))
            }
        }))
}
