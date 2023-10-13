use maud::{html, Markup};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ResetPasswordFormProps {
    pub token: Uuid,
    pub email: String,
    pub password_error: Option<String>,
    pub general_error: Option<String>,
}

pub fn reset_password_form(props: ResetPasswordFormProps) -> Markup {
    let ResetPasswordFormProps { token, email, password_error, general_error } = props;
    html! {
        form action="reset-password" method="post" class="auth-form-grid" {
            input
                type="text"
                name="token"
                id="token"
                value=(token.to_string())
                style="display:none;";
            label for="email" { "Email" }
            input
                type="email"
                name="email"
                id="email"
                placeholder="Email"
                value=(email)
                required;
            label for="password" { "Password" }
            input type="password" name="password" id="password" placeholder="Password" minlength="8" maxlength="255" required;
            @if let Some(password_error) = password_error {
                span class="error" { (password_error) }
            }
            label for="password_confirmation" { "Confirm Password" }
            input type="password" name="password_confirmation" id="password_confirmation" placeholder="Confirm Password" required;
            button type="submit" { "Reset password" }
            @if let Some(general_error) = general_error {
                span class="error" { (general_error) }
            }
        }
    }
}
