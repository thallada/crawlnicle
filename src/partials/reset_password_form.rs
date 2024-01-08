use maud::{html, Markup};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct ResetPasswordFormProps {
    pub token: Uuid,
    pub email: String,
    pub password_error: Option<String>,
    pub general_error: Option<String>,
}

pub fn reset_password_form(props: ResetPasswordFormProps) -> Markup {
    let ResetPasswordFormProps {
        token,
        email,
        password_error,
        general_error,
    } = props;
    html! {
        form
            action="/reset-password"
            method="post"
            hx-post="/reset-password"
            id="reset-password-form"
            class="my-4 flex flex-col gap-4"
        {
            input
                type="text"
                name="token"
                id="token"
                value=(token.to_string())
                class="hidden";
            div {
                label for="email" class="text-sm font-medium text-gray-700" { "Email" }
                input
                    type="email"
                    name="email"
                    id="email"
                    placeholder="Email"
                    value=(email)
                    required
                    class="w-full mt-1 p-2 bg-gray-50 border border-gray-300 shadow-sm rounded-md focus:ring focus:ring-blue-500 focus:border-blue-500 focus:ring-opacity-50";
            }
            div {
                label for="password" class="text-sm font-medium text-gray-700" { "Password" }
                input
                    type="password"
                    name="password"
                    id="password"
                    placeholder="Password"
                    minlength="8"
                    maxlength="255"
                    required
                    class="w-full mt-1 p-2 bg-gray-50 border border-gray-300 shadow-sm rounded-md focus:ring focus:ring-blue-500 focus:border-blue-500 focus:ring-opacity-50";
                @if let Some(password_error) = password_error {
                    span class="text-red-600" { (password_error) }
                }
            }
            div {
                label for="password_confirmation" class="text-sm font-medium text-gray-700" { "Confirm Password" }
                input
                    type="password"
                    name="password_confirmation"
                    id="password_confirmation"
                    placeholder="Confirm Password"
                    required
                    class="w-full mt-1 p-2 bg-gray-50 border border-gray-300 shadow-sm rounded-md focus:ring focus:ring-blue-500 focus:border-blue-500 focus:ring-opacity-50";
            }
            button type="submit" class="py-2 px-4 font-medium rounded-md border border-gray-200"{ "Reset password" }
            @if let Some(general_error) = general_error {
                span class="text-red-600" { (general_error) }
            }
        }
    }
}
