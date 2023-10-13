use ipnetwork::IpNetwork;
use lettre::message::{Mailbox, Message, MultiPart};
use lettre::{SmtpTransport, Transport};
use maud::html;
use tracing::error;

use crate::config::Config;
use crate::models::user::User;

pub fn send_password_reset_email(
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

        let email = match Message::builder()
            .from(config.email_from.clone())
            .to(mailbox)
            .subject("Your crawlnicle account password was reset")
            .multipart(MultiPart::alternative_plain_html(
                format!(
                    "Your crawlnicle account password was reset\n\nIf you did not perform this change, then this might indicate that your account has been compromised.\n\nRequest IP address: {}\nRequest user agent: {}",
                    request_ip,
                    request_user_agent.clone().unwrap_or_default()
                ),
                html! {
                    h1 { "Your crawlnicle account password was reset" }
                    p {
                        "If you did not perform this change, then this might indicate that your account has been compromised."
                    }
                    div {
                        "IP address: " (request_ip.to_string())
                    }
                    div {
                        "user agent: " (request_user_agent.unwrap_or_default())
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

        match mailer.send(&email) {
            Ok(_) => (),
            Err(err) => {
                error!("failed to send email: {}", err);
            }
        }
    });
}
