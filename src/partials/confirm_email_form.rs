use maud::{html, Markup};

use crate::models::user_email_verification_token::UserEmailVerificationToken;

pub fn confirm_email_form(token: Option<UserEmailVerificationToken>) -> Markup {
    html! {
        form action="/confirm-email" method="post" class="auth-form-grid" {
            @if let Some(token) = token {
                input type="text" name="token" id="token" value=(token.token_id) style="display:none;";
            } @else {
                label for="email" { "Email" }
                input type="email" name="email" id="email" placeholder="Email" required;
            }
            button type="submit" { "Resend confirmation email" }
        }
    }
}
