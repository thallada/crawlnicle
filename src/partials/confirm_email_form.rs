use maud::{html, Markup};

use crate::models::user_email_verification_token::UserEmailVerificationToken;

#[derive(Debug, Clone, Default)]
pub struct ConfirmEmailFormProps {
    pub token: Option<UserEmailVerificationToken>,
    pub email: Option<String>,
}

pub fn confirm_email_form(props: ConfirmEmailFormProps) -> Markup {
    let ConfirmEmailFormProps { token, email } = props;
    html! {
        form
            action="/confirm-email"
            method="post"
            hx-post="/confirm-email"
            id="confirm-email-form"
            class="auth-form-grid"
        {
            input
                type="text"
                name="token"
                id="token"
                value=(token.map(|t| t.token_id.to_string()).unwrap_or_default())
                style="display:none;";
            label for="email" { "Email" }
            input
                type="email"
                name="email"
                id="email"
                placeholder="Email"
                value=(email.unwrap_or_default())
                required;
            button type="submit" { "Resend confirmation email" }
        }
    }
}
