use maud::{html, Markup};

pub fn header() -> Markup {
    html! {
        header {
            nav {
                h1 { a href="/" data-turbo-frame="main" { "crawlnicle" } }
                ul {
                    li { a href="/feeds" data-turbo-frame="main" { "feeds" } }
                    li { a href="/log" data-turbo-frame="main" { "log" } }
                }
            }
        }
    }
}
