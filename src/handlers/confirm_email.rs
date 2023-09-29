use axum::extract::{Query, State};
use axum::response::Response;
use axum::TypedHeader;
use maud::html;
use serde::Deserialize;
use sqlx::PgPool;

use crate::error::{Error, Result};
use crate::htmx::HXTarget;
use crate::models::user::User;
use crate::models::user_email_verification_token::UserEmailVerificationToken;
use crate::partials::layout::Layout;
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
                                p { "Form with email input and button to resend goes here"}
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
                        p { "Form with button to resend goes here"}
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
                            "Thanks for verifying your email address."
                            a href="/" { "Return home" }
                        }
                    }
                }
            }))
    }
}
