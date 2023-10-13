use axum::extract::Query;
use axum::response::{IntoResponse, Response};
use axum::TypedHeader;
use axum::{extract::State, Form};
use axum_client_ip::SecureClientIp;
use headers::UserAgent;
use lettre::SmtpTransport;
use maud::html;
use serde::Deserialize;
use serde_with::serde_as;
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::htmx::HXTarget;
use crate::mailers::reset_password::send_password_reset_email;
use crate::models::user::UpdateUserPassword;
use crate::models::user_password_reset_token::UserPasswordResetToken;
use crate::partials::reset_password_form::{reset_password_form, ResetPasswordFormProps};
use crate::uuid::Base62Uuid;
use crate::{models::user::User, partials::layout::Layout};

#[serde_as]
#[derive(Deserialize)]
pub struct ResetPassword {
    pub token: Uuid,
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
}

#[derive(Deserialize)]
pub struct ResetPasswordQuery {
    pub token_id: Option<Base62Uuid>,
}

pub fn reset_password_page(
    hx_target: Option<TypedHeader<HXTarget>>,
    layout: Layout,
    form_props: ResetPasswordFormProps,
) -> Response {
    layout
        .with_subtitle("forgot password")
        .targeted(hx_target)
        .render(html! {
            div class="center-horizontal" {
                header class="center-text" {
                    h2 { "Reset Password" }
                }
                p {
                    "A password reset email will be sent if the email submitted matches an account in the system and the email is verfied. If your email is not verified, " a href="/confirm-email" { "please verify your email first" } "."
                }
                (reset_password_form(form_props))
            }
        })
        .into_response()
}

pub async fn get(
    State(pool): State<PgPool>,
    hx_target: Option<TypedHeader<HXTarget>>,
    layout: Layout,
    query: Query<ResetPasswordQuery>,
) -> Result<Response> {
    if let Some(token_id) = query.token_id {
        info!(token_id = %token_id.as_uuid(), "get with token_id");
        let token = match UserPasswordResetToken::get(&pool, token_id.as_uuid()).await {
            Ok(token) => token,
            Err(err) => {
                if let Error::NotFoundUuid(_, _) = err {
                    warn!(token_id = %token_id.as_uuid(), "token not found in database");
                    return Ok(layout
                        .with_subtitle("reset password")
                        .targeted(hx_target)
                        .render(html! {
                            div class="center-horizontal" {
                                header class="center-text" {
                                    h2 { "Password reset token not found" }
                                }
                                p class="readable-width" { "The reset password link has already been used or is invalid." }
                                p class="readable-width" { a href="/forgot-password" { "Follow this link to request a new password reset email" } "." }
                            }
                        }));
                }
                return Err(err);
            }
        };
        if token.expired() {
            warn!(token_id = %token.token_id, "token expired");
            Ok(layout
                .with_subtitle("reset password")
                .targeted(hx_target)
                .render(html! {
                    div class="center-horizontal" {
                        header class="center-text" {
                            h2 { "Password reset token is expired" }
                        }
                        p class="readable-width" { a href="/forgot-password" { "Follow this link to request a new password reset email" } ". The link in the email will be valid for 24 hours." }
                    }
                }))
        } else {
            info!(token_id = %token.token_id, "token valid, showing reset password form");
            let user = User::get(&pool, token.user_id).await?;
            Ok(layout
                .with_subtitle("reset password")
                .targeted(hx_target)
                .render(html! {
                    div class="center-horizontal" {
                        header class="center-text" {
                            h2 { "Reset Password" }
                        }
                        (reset_password_form(ResetPasswordFormProps {
                            token: token.token_id,
                            email: user.email,
                            password_error: None,
                            general_error: None,
                        }))
                    }
                }))
        }
    } else {
        Ok(layout
            .with_subtitle("reset password")
            .targeted(hx_target)
            .render(html! {
                div class="center-horizontal" {
                    header class="center-text" {
                        h2 { "Missing password reset token" }
                    }
                    p class="readable-width" { "Passwords can only be reset by requesting a password reset email and following the unique link within the email."}
                    p class="readable-width" { a href="/forgot-password" { "Follow this link to request a new password reset email" } "." }
                }
            }))
    }
}

pub async fn post(
    State(pool): State<PgPool>,
    State(mailer): State<SmtpTransport>,
    State(config): State<Config>,
    SecureClientIp(ip): SecureClientIp,
    hx_target: Option<TypedHeader<HXTarget>>,
    user_agent: Option<TypedHeader<UserAgent>>,
    layout: Layout,
    Form(reset_password): Form<ResetPassword>,
) -> Result<Response> {
    if reset_password.password != reset_password.password_confirmation {
        return Ok(layout
            .with_subtitle("reset password")
            .targeted(hx_target)
            .render(html! {
                div class="center-horizontal" {
                    header class="center-text" {
                        h2 { "Reset Password" }
                    }
                    (reset_password_form(ResetPasswordFormProps {
                        token: reset_password.token,
                        email: reset_password.email,
                        password_error: Some("passwords do not match".to_string()),
                        general_error: None,
                    }))
                }
            }));
    }
    let token = match UserPasswordResetToken::get(&pool, reset_password.token).await {
        Ok(token) => token,
        Err(err) => {
            if let Error::NotFoundUuid(_, _) = err {
                warn!(token_id = %reset_password.token, "token not found in database");
                return Ok(layout
                    .with_subtitle("reset password")
                    .targeted(hx_target)
                    .render(html! {
                        div class="center-horizontal" {
                            header class="center-text" {
                                h2 { "Reset Password" }
                            }
                            (reset_password_form(ResetPasswordFormProps {
                                token: reset_password.token,
                                email: reset_password.email,
                                password_error: None,
                                general_error: Some("token not found".to_string()),
                            }))
                            p class="error readable-width" { "The reset password link has already been used or is invalid." }
                            p class="readable-width" { a href="/forgot-password" { "Follow this link to request a new password reset email" } "." }
                        }
                    }));
            }
            return Err(err);
        }
    };
    if token.expired() {
        warn!(token_id = %token.token_id, "token expired");
        return Ok(layout
            .with_subtitle("reset password")
            .targeted(hx_target)
            .render(html! {
                div class="center-horizontal" {
                    header class="center-text" {
                        h2 { "Reset Password" }
                    }
                    (reset_password_form(ResetPasswordFormProps {
                        token: reset_password.token,
                        email: reset_password.email,
                        password_error: None,
                        general_error: Some("token expired".to_string()),
                    }))
                    p class="error readable-width" { "The reset password link has expired." }
                    p class="readable-width" { a href="/forgot-password" { "Follow this link to request a new password reset email" } ". The link in the email will be valid for 24 hours." }
                }
            }));
    }
    let user = match User::get(&pool, token.user_id).await {
        Ok(user) => user,
        Err(err) => {
            if let Error::NotFoundString(_, _) = err {
                info!(user_id = %token.user_id, email = reset_password.email, "invalid token user_id");
                return Ok(layout
                    .with_subtitle("reset password")
                    .targeted(hx_target)
                    .render(html! {
                        div class="center-horizontal" {
                            header class="center-text" {
                                h2 { "Reset Password" }
                            }
                            (reset_password_form(ResetPasswordFormProps {
                                token: reset_password.token,
                                email: reset_password.email,
                                password_error: None,
                                general_error: Some("user not found".to_string()),
                            }))
                            p class="error readable-width" { "The user associated with this password reset could not be found." }
                            p class="readable-width" { a href="/forgot-password" { "Follow this link to request a new password reset email" } "." }
                        }
                    }));
            } else {
                return Err(err);
            }
        }
    };
    info!(user_id = %user.user_id, "user exists with verified email, resetting password");
    // TODO: do both in transaction
    UserPasswordResetToken::delete(&pool, reset_password.token).await?;
    let user = match user
        .update_password(
            &pool,
            UpdateUserPassword {
                password: reset_password.password,
            },
        )
        .await
    {
        Ok(user) => user,
        Err(err) => {
            if let Error::InvalidEntity(validation_errors) = err {
                let field_errors = validation_errors.field_errors();
                return Ok(layout
                    .with_subtitle("reset password")
                    .targeted(hx_target)
                    .render(html! {
                        div class="center-horizontal" {
                            header class="center-text" {
                                h2 { "Reset Password" }
                            }
                            (reset_password_form(ResetPasswordFormProps {
                                token: reset_password.token,
                                email: reset_password.email,
                                password_error: field_errors.get("password").map(|&errors| {
                                    errors
                                        .iter()
                                        .filter_map(|error| error.message.clone().map(|m| m.to_string()))
                                        .collect::<Vec<String>>()
                                        .join(", ")
                                }),
                                general_error: None,
                            }))
                        }
                    }));
            }
            return Err(err);
        }
    };
    send_password_reset_email(
        mailer,
        config,
        user,
        ip.into(),
        user_agent.map(|ua| ua.to_string()),
    );
    Ok(layout
        .with_subtitle("reset password")
        .targeted(hx_target)
        .render(html! {
            div class="center-horizontal" {
                header class="center-text" {
                    h2 { "Password reset!" }
                }
                p class="readable-width" {
                    "Your password has been reset. "
                    a href="/" { "Return home" }
                }
            }
        }))
}
