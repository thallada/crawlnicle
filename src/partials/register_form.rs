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
        form
            action="/register"
            method="post"
            hx-post="/register"
            hx-target="#register-form"
            hx-swap="outerHTML"
            id="register-form"
            class="my-4 flex flex-col gap-4"
        {
            div {
                label for="email" class="text-sm font-medium text-gray-700" { "Email *" }
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
                label for="name" class="text-sm font-medium text-gray-700" { (PreEscaped("Name &nbsp;")) }
                input
                    type="text"
                    name="name"
                    id="name"
                    value=(name.unwrap_or_default())
                    placeholder="Name"
                    maxlength="255"
                    class="w-full mt-1 p-2 bg-gray-50 border border-gray-300 shadow-sm rounded-md focus:ring focus:ring-blue-500 focus:border-blue-500 focus:ring-opacity-50";
                @if let Some(name_error) = name_error {
                    span class="text-red-600" { (name_error) }
                }
            }
            div {
                label for="email" class="text-sm font-medium text-gray-700" { "Password *" }
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
                label for="password_confirmation" class="text-sm font-medium text-gray-700" { "Confirm Password *" }
                input
                    type="password"
                    name="password_confirmation"
                    id="password_confirmation"
                    placeholder="Confirm Password"
                    required
                    class="w-full mt-1 p-2 bg-gray-50 border border-gray-300 shadow-sm rounded-md focus:ring focus:ring-blue-500 focus:border-blue-500 focus:ring-opacity-50";
            }
            button type="submit" class="py-2 px-4 font-medium rounded-md border border-gray-200"{ "Submit" }
            @if let Some(general_error) = general_error {
                span class="text-red-600" { (general_error) }
            }
        }
    }
}
