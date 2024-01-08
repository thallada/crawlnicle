use maud::{html, Markup};

use crate::partials::link::{link, LinkProps};

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
                    required
                    class="w-full mt-1 p-2 bg-gray-50 border border-gray-300 shadow-sm rounded-md focus:ring focus:ring-blue-500 focus:border-blue-500 focus:ring-opacity-50";
                @if let Some(email_error) = email_error {
                    span class="text-red-600" { (email_error) }
                }
            }
            div {
                label for="pwassword" class="text-sm font-medium text-gray-700" { "Password" }
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
            button type="submit" class="py-2 px-4 font-medium rounded-md border border-gray-200" { "Submit" }
            @if let Some(general_error) = general_error {
                span class="text-red-600" { (general_error) }
            }
            div class="ml-auto" {
                (link(LinkProps {
                    destination: "/forgot-password",
                    title: "Forgot password",
                    reset_htmx_target: true,
                }))
            }
        }
    }
}
