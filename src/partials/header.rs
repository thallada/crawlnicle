use maud::{html, Markup};

use crate::models::user::User;
use crate::partials::user_name::user_name;

pub fn header(title: &str, user: Option<User>) -> Markup {
    html! {
        header class="header" {
            nav {
                h1 { a href="/" { (title) } }
                ul {
                    li { a href="/feeds" { "feeds" } }
                    li { a href="/log" { "log" } }
                }
                div class="auth" {
                    @if let Some(user) = user {
                        (user_name(user))
                        span { " | " }
                        a href="/logout" { "logout" }
                    } @else {
                        a href="/login" { "login" }
                        span { " | " }
                        a href="/register" { "register" }
                    }
                }
            }
        }
    }
}
