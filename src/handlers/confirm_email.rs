use axum::extract::{Query, State};
use axum::response::Response;
use axum::Form;
use axum_extra::TypedHeader;
use lettre::SmtpTransport;
use maud::{html, Markup};
use serde::Deserialize;
use serde_with::{serde_as, NoneAsEmptyString};
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::htmx::HXTarget;
use crate::mailers::email_verification::send_confirmation_email;
use crate::models::user::User;
use crate::models::user_email_verification_token::UserEmailVerificationToken;
use crate::partials::confirm_email_form::{confirm_email_form, ConfirmEmailFormProps};
use crate::partials::layout::Layout;
use crate::partials::link::{link, LinkProps};
use crate::uuid::Base62Uuid;

#[derive(Deserialize)]
pub struct ConfirmEmailQuery {
    pub token_id: Option<Base62Uuid>,
}

#[derive(Debug, Default)]
pub struct ConfirmEmailPageProps<'a> {
    pub hx_target: Option<TypedHeader<HXTarget>>,
    pub layout: Layout,
    pub form_props: ConfirmEmailFormProps,
    pub header: Option<&'a str>,
    pub desc: Option<Markup>,
}

pub fn confirm_email_page(
    ConfirmEmailPageProps {
        hx_target,
        layout,
        form_props,
        header,
        desc,
    }: ConfirmEmailPageProps,
) -> Response {
    layout
        .with_subtitle("confirm email")
        .targeted(hx_target)
        .render(html! {
            div class="w-fit mx-auto" {
                header class="text-center" {
                    h2 class="mb-4 text-2xl font-medium" {
                        (header.unwrap_or("Confirm your email address"))
                    }
                }
                @if let Some(desc) = desc {
                    (desc)
                } @else {
                    p class="my-4 max-w-prose" {
                        "Enter your email to resend the confirmation email. If you don't have an account yet, create one "
                        (link(LinkProps { destination: "/register", title: "here", ..Default::default() }))
                        "."
                    }
                }
                (confirm_email_form(form_props))
            }
        })
}

pub async fn get(
    State(pool): State<PgPool>,
    auth: AuthSession,
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
                    return Ok(confirm_email_page(ConfirmEmailPageProps {
                        hx_target,
                        layout,
                        form_props: ConfirmEmailFormProps::default(),
                        header: Some("Email verification token not found"),
                        ..Default::default()
                    }));
                }
                return Err(err);
            }
        };
        if token.expired() {
            warn!(token_id = %token.token_id, "token expired");
            Ok(confirm_email_page(ConfirmEmailPageProps {
                hx_target,
                layout,
                form_props: ConfirmEmailFormProps {
                    token: Some(token),
                    ..Default::default()
                },
                header: Some("Email verification token is expired"),
                desc: Some(html! {
                    p class="my-4 max-w-prose" {
                        "Click the button below to resend a new confirmation email. The link in the email will be valid for another 24 hours."
                    }
                }),
            }))
        } else {
            info!(token_id = %token.token_id, "token valid, verifying email");
            User::verify_email(&pool, token.user_id).await?;
            UserEmailVerificationToken::delete(&pool, token.token_id).await?;
            Ok(layout
                .with_subtitle("confirm email")
                .targeted(hx_target)
                .render(html! {
                    div class="w-fit mx-auto" {
                        header class="text-center" {
                            h2 class="mb-4 text-2xl font-medium" { "Your email is now confirmed!" }
                        }
                        p class="my-4 max-w-prose" {
                            "Thanks for verifying your email address. "
                            (link(LinkProps { destination: "/", title: "Return home", ..Default::default() }))
                        }
                    }
                }))
        }
    } else {
        Ok(confirm_email_page(ConfirmEmailPageProps {
            hx_target,
            layout,
            form_props: ConfirmEmailFormProps {
                email: auth.user.map(|u| u.email),
                ..Default::default()
            },
            ..Default::default()
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
                div class="w-fit mx-auto" {
                    header class="text-center" {
                        h2 class="mb-4 text-2xl font-medium" { "Resent confirmation email" }
                    }
                    p class="my-4 max-w-prose" {
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
                div class="w-fit mx-auto" {
                    header class="text-center" {
                        h2 class="mb-4 text-2xl font-medium" { "Resent confirmation email" }
                    }
                    p class="my-4 max-w-prose" {
                        "If the email you entered matched an existing account, then a confirmation email was sent. Please follow the link sent in the email."
                    }
                }
            }));
    }
    Ok(confirm_email_page(ConfirmEmailPageProps {
        hx_target,
        layout,
        form_props: ConfirmEmailFormProps::default(),
        header: Some("Email verification token not found"),
        desc: Some(html! {
            p class="my-4 max-w-prose" {
            "Enter your email to resend the confirmation email. If you don't have an account yet, create one "
            (link(LinkProps { destination: "/register", title: "here", ..Default::default() }))
            "."
            }
        }),
    }))
}
