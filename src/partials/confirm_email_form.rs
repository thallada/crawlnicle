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
            class="my-4 flex flex-col gap-4"
        {
            input
                type="text"
                name="token"
                id="token"
                value=(token.map(|t| t.token_id.to_string()).unwrap_or_default())
                class="hidden";
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
            }
            button type="submit" class="py-2 px-4 font-medium rounded-md border border-gray-200"{ "Resend confirmation email" }
        }
    }
}
