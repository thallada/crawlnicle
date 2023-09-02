use maud::{html, Markup};

pub fn footer() -> Markup {
    html! {
        footer class="footer" {
            hr;
            "Made by " a href="https://www.hallada.net" { "Tyler Hallada" }"."
        }
    }
}
