use maud::{html, Markup};

pub fn header() -> Markup {
    html! {
        header {
            nav {
                h1 { a href="/" { "crawlnicle" } }
                ul {
                    li { a href="/feeds" { "feeds" } }
                }
            }
        }
    }
}
