use std::time::Duration;

use chrono::Utc;
use ipnetwork::IpNetwork;
use lettre::message::{Mailbox, Message, MultiPart};
use lettre::{SmtpTransport, Transport};
use maud::html;
use sqlx::PgPool;
use tracing::error;
use uuid::Uuid;

use crate::config::Config;
use crate::models::user::User;
use crate::models::user_password_reset_token::{CreatePasswordResetToken, UserPasswordResetToken};
use crate::uuid::Base62Uuid;

// TODO: put in config
const PASSWORD_RESET_TOKEN_EXPIRATION: Duration = Duration::from_secs(24 * 60 * 60);

pub fn send_forgot_password_email(
    pool: PgPool,
    mailer: SmtpTransport,
    config: Config,
    user: User,
    request_ip: IpNetwork,
    request_user_agent: Option<String>,
) {
    tokio::spawn(async move {
        let user_email_address = match user.email.parse() {
            Ok(address) => address,
            Err(err) => {
                error!("failed to parse email address: {}", err);
                return;
            }
        };
        let mailbox = Mailbox::new(user.name.clone(), user_email_address);
        let token = match UserPasswordResetToken::create(
            &pool,
            CreatePasswordResetToken {
                token_id: Uuid::new_v4(), // cyptographically-secure random uuid
                user_id: user.user_id,
                request_ip,
                request_user_agent,
                expires_at: Utc::now() + PASSWORD_RESET_TOKEN_EXPIRATION,
            },
        )
        .await
        {
            Ok(token) => token,
            Err(err) => {
                error!("failed to create user password reset token: {}", err);
                return;
            }
        };
        let mut password_reset_link = config.public_url.clone();
        password_reset_link.set_path("reset-password");
        password_reset_link
            .query_pairs_mut()
            .append_pair("token_id", &Base62Uuid::from(token.token_id).to_string());
        let password_reset_link = password_reset_link.as_str();

        let email = match Message::builder()
            .from(config.email_from.clone())
            .to(mailbox)
            .subject("Reset your crawlnicle account password")
            .multipart(MultiPart::alternative_plain_html(
                format!(
                    "Reset your crawlnicle account password\n\nA password reset has been requested for your crawlnicle account. If you did not request this, please ignore this email.\n\nRequest IP address: {}\nRequest user agent: {}\n\nClick here to reset your password: {}",
                    token.request_ip,
                    token.request_user_agent.clone().unwrap_or_default(),
                    password_reset_link
                ),
                html! {
                    h1 { "Reset your crawlnicle account password" }
                    p {
                        "A password reset has been requested for your crawlnicle account. If you did not request this, please ignore this email."
                    }
                    div {
                        "IP address: " (token.request_ip.to_string())
                    }
                    div {
                        "user agent: " (token.request_user_agent.unwrap_or_default())
                    }
                    p {
                        a href=(password_reset_link) { "Click here to reset your password" }
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
