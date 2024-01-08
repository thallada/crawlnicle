use maud::{html, Markup};

use crate::partials::link::{link, LinkProps};

pub fn footer() -> Markup {
    html! {
        footer class="text-center mt-16 mb-2" {
            hr class="w-12 mx-auto mb-4";
            "Made by " (link(LinkProps {
                destination: "https://www.hallada.net",
                title: "Tyler Hallada",
                ..Default::default()
            })) "."
        }
    }
}
