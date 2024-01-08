use maud::{html, Markup};

use crate::models::user::User;
use crate::partials::link::{home_link, link, LinkProps};
use crate::partials::user_name::user_name;

pub fn header(title: &str, user: Option<User>) -> Markup {
    html! {
        header {
            nav class="flex flex-row items-baseline justify-between" {
                div class="flex flex-row items-baseline gap-4" {
                    h1 {
                        (home_link(LinkProps {
                            destination: "/",
                            title,
                            ..Default::default()
                        }))
                    }
                    ul class="flex flex-row list-none gap-4" {
                        li { (link(LinkProps { destination: "/feeds", title: "feeds", ..Default::default() })) }
                        li { (link(LinkProps { destination: "/log", title: "log", ..Default::default() })) }
                    }
                }
                div class="auth" {
                    @if let Some(user) = user {
                        (user_name(user.clone()))
                        @if !user.email_verified {
                            span { " (" }
                            (link(LinkProps { destination: "/confirm-email", title: "unverified", ..Default::default() }))
                            span { ")" }
                        }
                        span { " | " }
                        (link(LinkProps { destination: "/logout", title: "logout", ..Default::default() }))
                    } @else {
                        (link(LinkProps { destination: "/login", title: "login", ..Default::default() }))
                        span { " | " }
                        (link(LinkProps { destination: "/register", title: "register", ..Default::default() }))
                    }
                }
            }
        }
    }
}
