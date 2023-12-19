use axum::extract::Query;
use axum::response::Response;
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

#[derive(Debug, Default)]
pub struct InvalidTokenPageProps<'a> {
    pub hx_target: Option<TypedHeader<HXTarget>>,
    pub layout: Layout,
    pub header: Option<&'a str>,
    pub desc: Option<&'a str>,
}

pub fn invalid_token_page(
    InvalidTokenPageProps {
        hx_target,
        layout,
        header,
        desc,
    }: InvalidTokenPageProps,
) -> Response {
    layout
        .with_subtitle("reset password")
        .targeted(hx_target)
        .render(html! {
            div class="center-horizontal" {
                header class="center-text" {
                    h2 { (header.unwrap_or("Reset Password")) }
                }
                @if let Some(desc) = desc {
                    p class="readable-width" { (desc) }
                }
                p class="readable-width" {
                    a href="/forgot-password" {
                        "Follow this link to request a new password reset email"
                    }
                    "."
                }
            }
        })
}

#[derive(Debug, Default)]
pub struct ResetPasswordPageProps<'a> {
    pub hx_target: Option<TypedHeader<HXTarget>>,
    pub layout: Layout,
    pub form_props: ResetPasswordFormProps,
    pub header: Option<&'a str>,
    pub post_form_error: Option<&'a str>,
}

pub fn reset_password_page(
    ResetPasswordPageProps {
        hx_target,
        layout,
        form_props,
        header,
        post_form_error,
    }: ResetPasswordPageProps,
) -> Response {
    layout
        .with_subtitle("reset password")
        .targeted(hx_target)
        .render(html! {
            div class="center-horizontal" {
                header class="center-text" {
                    h2 { (header.unwrap_or("Reset Password")) }
                }
                p class="readable-width" {
                    "A password reset email will be sent if the email submitted matches an account in the system and the email is verfied. If your email is not verified, " a href="/confirm-email" { "please verify your email first" } "."
                }
                (reset_password_form(form_props))
                @if let Some(post_form_error) = post_form_error {
                    p class="error readable-width" { (post_form_error) }
                    p class="readable-width" {
                        a href="/forgot-password" {
                            "Follow this link to request a new password reset email"
                        }
                        ". The link in the email will be valid for 24 hours."
                    }
                }
            }
        })
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
                    return Ok(invalid_token_page(InvalidTokenPageProps {
                        hx_target,
                        layout,
                        header: Some("Password reset token not found"),
                        desc: Some("The reset password link has already been used or is invalid."),
                    }));
                }
                return Err(err);
            }
        };
        if token.expired() {
            warn!(token_id = %token.token_id, "token expired");
            Ok(invalid_token_page(InvalidTokenPageProps {
                hx_target,
                layout,
                header: Some("Password reset token expired"),
                ..Default::default()
            }))
        } else {
            info!(token_id = %token.token_id, "token valid, showing reset password form");
            let user = User::get(&pool, token.user_id).await?;
            Ok(reset_password_page(ResetPasswordPageProps {
                hx_target,
                layout,
                form_props: ResetPasswordFormProps {
                    token: token.token_id,
                    email: user.email,
                    ..Default::default()
                },
                ..Default::default()
            }))
        }
    } else {
        Ok(invalid_token_page(InvalidTokenPageProps {
            hx_target,
            layout,
            header: Some("Missing password reset token"),
            desc: Some("Passwords can only be reset by requesting a password reset email and following the unique link within the email."),
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
        return Ok(reset_password_page(ResetPasswordPageProps {
            hx_target,
            layout,
            form_props: ResetPasswordFormProps {
                token: reset_password.token,
                email: reset_password.email,
                password_error: Some("passwords do not match".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }));
    }
    let token = match UserPasswordResetToken::get(&pool, reset_password.token).await {
        Ok(token) => token,
        Err(err) => {
            if let Error::NotFoundUuid(_, _) = err {
                warn!(token_id = %reset_password.token, "token not found in database");
                return Ok(reset_password_page(ResetPasswordPageProps {
                    hx_target,
                    layout,
                    form_props: ResetPasswordFormProps {
                        token: reset_password.token,
                        email: reset_password.email,
                        general_error: Some("token not found".to_string()),
                        ..Default::default()
                    },
                    post_form_error: Some(
                        "The reset password link has already been used or is invalid.",
                    ),
                    ..Default::default()
                }));
            }
            return Err(err);
        }
    };
    if token.expired() {
        warn!(token_id = %token.token_id, "token expired");
        return Ok(reset_password_page(ResetPasswordPageProps {
            hx_target,
            layout,
            form_props: ResetPasswordFormProps {
                token: reset_password.token,
                email: reset_password.email,
                general_error: Some("token expired".to_string()),
                ..Default::default()
            },
            post_form_error: Some("The reset password link has expired."),
            ..Default::default()
        }));
    }
    let user = match User::get(&pool, token.user_id).await {
        Ok(user) => user,
        Err(err) => {
            if let Error::NotFoundString(_, _) = err {
                info!(user_id = %token.user_id, email = reset_password.email, "invalid token user_id");
                return Ok(reset_password_page(ResetPasswordPageProps {
                    hx_target,
                    layout,
                    form_props: ResetPasswordFormProps {
                        token: reset_password.token,
                        email: reset_password.email,
                        general_error: Some("user not found".to_string()),
                        ..Default::default()
                    },
                    post_form_error: Some(
                        "The user associated with this password reset could not be found.",
                    ),
                    ..Default::default()
                }));
            } else {
                return Err(err);
            }
        }
    };
    info!(user_id = %user.user_id, "user exists with verified email, resetting password");
    let mut tx = pool.begin().await?;
    UserPasswordResetToken::delete(tx.as_mut(), reset_password.token).await?;
    let user = match user
        .update_password(
            tx.as_mut(),
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
                return Ok(reset_password_page(ResetPasswordPageProps {
                    hx_target,
                    layout,
                    form_props: ResetPasswordFormProps {
                        token: reset_password.token,
                        email: reset_password.email,
                        password_error: field_errors.get("password").map(|&errors| {
                            errors
                                .iter()
                                .filter_map(|error| error.message.clone().map(|m| m.to_string()))
                                .collect::<Vec<String>>()
                                .join(", ")
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
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
    tx.commit().await?;
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
