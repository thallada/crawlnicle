use maud::{html, Markup};

#[derive(Debug, Clone, Default)]
pub struct ForgotPasswordFormProps {
    pub email: Option<String>,
    pub email_error: Option<String>,
}

pub fn forgot_password_form(props: ForgotPasswordFormProps) -> Markup {
    let ForgotPasswordFormProps { email, email_error } = props;
    html! {
        form action="forgot-password" method="post" class="auth-form-grid" {
            label for="email" { "Email" }
            input
                type="email"
                name="email"
                id="email"
                placeholder="Email"
                value=(email.unwrap_or_default())
                required;
            @if let Some(email_error) = email_error {
                span class="error" { (email_error) }
            }
            button type="submit" { "Send password reset email" }
        }
    }
}
