use axum::response::{IntoResponse, Response};
use axum::{extract::State, Form};
use axum_client_ip::SecureClientIp;
use axum_extra::TypedHeader;
use headers::UserAgent;
use lettre::SmtpTransport;
use maud::html;
use serde::Deserialize;
use serde_with::serde_as;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::auth::AuthSession;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::htmx::HXTarget;
use crate::mailers::forgot_password::send_forgot_password_email;
use crate::partials::forgot_password_form::{forgot_password_form, ForgotPasswordFormProps};
use crate::{models::user::User, partials::layout::Layout};

#[serde_as]
#[derive(Deserialize)]
pub struct ForgotPassword {
    email: String,
}

pub fn forgot_password_page(
    hx_target: Option<TypedHeader<HXTarget>>,
    layout: Layout,
    form_props: ForgotPasswordFormProps,
) -> Response {
    layout
        .with_subtitle("forgot password")
        .targeted(hx_target)
        .render(html! {
            div class="w-fit mx-auto" {
                header class="text-center" {
                    h2 class="mb-4 text-2xl font-medium" { "Forgot Password" }
                }
                p class="my-4 max-w-prose" {
                    "A password reset email will be sent if the email submitted matches an account in the system and the email is verfied. If your email is not verified, " a href="/confirm-email" { "please verify your email first" } "."
                }
                (forgot_password_form(form_props))
            }
        })
        .into_response()
}

pub fn confirm_forgot_password_sent_page(
    hx_target: Option<TypedHeader<HXTarget>>,
    layout: Layout,
) -> Response {
    layout
        .with_subtitle("forgot password")
        .targeted(hx_target)
        .render(html! {
            div class="w-fit mx-auto" {
                header class="text-center" {
                    h2 class="mb-4 text-2xl font-medium" { "Reset password email sent" }
                }
                p class="my-4 max-w-prose" {
                    "If the email you entered matched an existing account with a verified email, then a password reset email was sent. Please follow the link sent in the email."
                }
            }
        })
        .into_response()
}

pub async fn get(
    auth: AuthSession,
    hx_target: Option<TypedHeader<HXTarget>>,
    layout: Layout,
) -> Result<Response> {
    Ok(forgot_password_page(
        hx_target,
        layout,
        ForgotPasswordFormProps {
            email: auth.user.map(|u| u.email),
            email_error: None,
        },
    ))
}

pub async fn post(
    State(pool): State<PgPool>,
    State(mailer): State<SmtpTransport>,
    State(config): State<Config>,
    SecureClientIp(ip): SecureClientIp,
    hx_target: Option<TypedHeader<HXTarget>>,
    user_agent: Option<TypedHeader<UserAgent>>,
    layout: Layout,
    Form(forgot_password): Form<ForgotPassword>,
) -> Result<Response> {
    let user: User = match User::get_by_email(&pool, forgot_password.email.clone()).await {
        Ok(user) => user,
        Err(err) => {
            if let Error::NotFoundString(_, _) = err {
                info!(email = forgot_password.email, "invalid email");
                return Ok(confirm_forgot_password_sent_page(hx_target, layout));
            } else {
                return Err(err);
            }
        }
    };
    if user.email_verified {
        info!(user_id = %user.user_id, "user exists with verified email, sending password reset email");
        send_forgot_password_email(
            pool,
            mailer,
            config,
            user,
            ip.into(),
            user_agent.map(|ua| ua.to_string()),
        );
    } else {
        warn!(user_id = %user.user_id, "user exists with unverified email, skip sending password reset email");
    }
    Ok(confirm_forgot_password_sent_page(hx_target, layout))
}
