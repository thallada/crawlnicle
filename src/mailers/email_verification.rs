use std::time::Duration;

use chrono::Utc;
use lettre::message::{Mailbox, Message, MultiPart};
use lettre::{SmtpTransport, Transport};
use maud::html;
use sqlx::PgPool;
use tracing::error;

use crate::config::Config;
use crate::models::user::User;
use crate::models::user_email_verification_token::{
    CreateUserEmailVerificationToken, UserEmailVerificationToken,
};
use crate::uuid::Base62Uuid;

// TODO: put in config
const USER_EMAIL_VERIFICATION_TOKEN_EXPIRATION: Duration = Duration::from_secs(24 * 60 * 60);

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
        let mut confirm_link = config
            .public_url
            .clone();
        confirm_link.set_path("confirm-email");
        confirm_link.query_pairs_mut()
            .append_pair("token_id", &Base62Uuid::from(token.token_id).to_string());
        let confirm_link = confirm_link.as_str();

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
