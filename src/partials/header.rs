use maud::{html, Markup};

pub fn header(title: &str) -> Markup {
    html! {
        header class="header" {
            nav {
                h1 { a href="/" data-turbo-frame="main" { (title) } }
                ul {
                    li { a href="/feeds" data-turbo-frame="main" { "feeds" } }
                    li { a href="/log" data-turbo-frame="main" { "log" } }
                }
            }
        }
    }
}
