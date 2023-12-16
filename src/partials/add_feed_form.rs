use maud::{html, Markup};

pub fn add_feed_form() -> Markup {
    html! {
        form
            action="/feed"
            method="post"
            id="add-feed-form"
            hx-post="/feed"
            hx-target="#add-feed-form"
            hx-swap="outerHTML"
            class="feed-form"
        {
            div class="form-grid" {
                label for="url" { "URL: " }
                input
                    type="text"
                    id="url"
                    name="url"
                    placeholder="https://example.com/feed.xml"
                    required="true";
                button type="submit" { "Add Feed" }
            }
        }
    }
}
