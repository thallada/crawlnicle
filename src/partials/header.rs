use maud::{html, Markup};

pub fn header(title: &str) -> Markup {
    html! {
        header class="header" {
            nav {
                h1 { a href="/" { (title) } }
                ul {
                    li { a href="/feeds" { "feeds" } }
                    li { a href="/log" { "log" } }
                }
            }
        }
    }
}
