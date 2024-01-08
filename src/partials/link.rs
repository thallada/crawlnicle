use maud::{html, Markup};

#[derive(Debug, Default)]
pub struct LinkProps<'a> {
    pub destination: &'a str,
    pub title: &'a str,
    pub reset_htmx_target: bool,
}

pub fn link(
    LinkProps {
        destination,
        title,
        reset_htmx_target,
    }: LinkProps<'_>,
) -> Markup {
    let hx_target = if reset_htmx_target {
        Some("#main-content")
    } else {
        None
    };
    let hx_swap = if reset_htmx_target {
        Some("unset")
    } else {
        None
    };
    html! {
        a
            href=(destination)
            hx-target=[hx_target]
            hx-swap=[hx_swap]
            class="text-blue-600 visited:text-purple-600 hover:underline"
        {
            (title)
        }
    }
}

pub fn home_link(
    LinkProps {
        destination, title, ..
    }: LinkProps<'_>,
) -> Markup {
    html! {
        a href=(destination) class="text-2xl text-blue-600 visited:text-purple-600 hover:underline" { (title) }
    }
}
