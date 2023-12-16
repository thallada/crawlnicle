use maud::{html, Markup};

#[derive(Debug, Default)]
pub struct LoginFormProps {
    pub email: Option<String>,
    pub email_error: Option<String>,
    pub password_error: Option<String>,
    pub general_error: Option<String>,
}

pub fn login_form(props: LoginFormProps) -> Markup {
    let LoginFormProps {
        email,
        email_error,
        password_error,
        general_error,
    } = props;
    html! {
        form
            action="/login"
            method="post"
            hx-post="/login"
            hx-target="#login-form"
            hx-swap="outerHTML"
            id="login-form"
            class="auth-form-grid"
        {
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
            label for="email" { "Password" }
            input
                type="password"
                name="password"
                id="password"
                placeholder="Password"
                minlength="8"
                maxlength="255"
                required;
            @if let Some(password_error) = password_error {
                span class="error" { (password_error) }
            }
            button type="submit" { "Submit" }
            @if let Some(general_error) = general_error {
                span class="error" { (general_error) }
            }
            a href="/forgot-password" hx-target="#main-content" hx-swap="unset" class="forgot-password" {
                "Forgot password"
            }
        }
    }
}
