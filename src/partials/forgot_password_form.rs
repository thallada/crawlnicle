use maud::{html, Markup};

#[derive(Debug, Clone, Default)]
pub struct ForgotPasswordFormProps {
    pub email: Option<String>,
    pub email_error: Option<String>,
}

pub fn forgot_password_form(props: ForgotPasswordFormProps) -> Markup {
    let ForgotPasswordFormProps { email, email_error } = props;
    html! {
        form
            action="/forgot-password"
            method="post"
            hx-post="/forgot-password"
            id="forgot-password-form"
            class="my-4 flex flex-col gap-4"
        {
            div {
                label for="email" class="text-sm font-medium text-gray-700" { "Email" }
                input
                    type="email"
                    name="email"
                    id="email"
                    placeholder="Email"
                    value=(email.unwrap_or_default())
                    class="w-full mt-1 p-2 bg-gray-50 border border-gray-300 shadow-sm rounded-md focus:ring focus:ring-blue-500 focus:border-blue-500 focus:ring-opacity-50";
                @if let Some(email_error) = email_error {
                    span class="text-red-600" { (email_error) }
                }
            }
            button type="submit" class="py-2 px-4 font-medium rounded-md border border-gray-200" {
                "Send password reset email"
            }
        }
    }
}
