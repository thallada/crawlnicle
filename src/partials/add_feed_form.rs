use maud::{html, Markup};

pub fn add_feed_form() -> Markup {
    html! {
        form hx-post="/feed" class="feed-form" {
            div class="form-grid" {
                label for="url" { "URL: " }
                input type="text" id="url" name="url" placeholder="https://example.com/feed.xml" required="true";
                button type="submit" { "Add Feed" }
            }
        }
    }
}
