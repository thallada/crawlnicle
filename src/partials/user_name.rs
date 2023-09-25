use maud::{html, Markup};

use crate::models::user::User;

pub fn user_name(user: User) -> Markup {
    let name = user.name.unwrap_or(user.email);
    html! {
        a href="/account" { (name) }
    }
}
