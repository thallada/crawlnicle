use maud::{html, Markup, PreEscaped};

#[derive(Debug, Default)]
pub struct RegisterFormProps {
    pub email: Option<String>,
    pub name: Option<String>,
    pub email_error: Option<String>,
    pub name_error: Option<String>,
    pub password_error: Option<String>,
    pub general_error: Option<String>,
}

pub fn register_form(props: RegisterFormProps) -> Markup {
    let RegisterFormProps {
        email,
        name,
        email_error,
        name_error,
        password_error,
        general_error,
    } = props;
    html! {
        form action="/register" method="post" class="auth-form-grid" {
            label for="email" { "Email *" }
            input type="email" name="email" id="email" placeholder="Email" value=(email.unwrap_or_default()) required;
            @if let Some(email_error) = email_error {
                span class="error" { (email_error) }
            }
            label for="name" { (PreEscaped("Name &nbsp;")) }
            input type="text" name="name" id="name" value=(name.unwrap_or_default()) placeholder="Name" maxlength="255";
            @if let Some(name_error) = name_error {
                span class="error" { (name_error) }
            }
            label for="email" { "Password *" }
            input type="password" name="password" id="password" placeholder="Password" minlength="8" maxlength="255" required;
            @if let Some(password_error) = password_error {
                span class="error" { (password_error) }
            }
            label for="password_confirmation" { "Confirm Password *" }
            input type="password" name="password_confirmation" id="password_confirmation" placeholder="Confirm Password" required;
            button type="submit" { "Submit" }
            @if let Some(general_error) = general_error {
                span class="error" { (general_error) }
            }
        }
    }
}
