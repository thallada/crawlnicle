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
            class="flex flex-row gap-6 items-end justify-between"
        {
            // TODO: make into an input partial component
            div class="grow w-full" {
                label for="url" class="text-sm font-medium text-gray-700" { "URL" }
                input
                    type="text"
                    id="url"
                    name="url"
                    placeholder="https://example.com/feed.xml"
                    required="true"
                    class="w-full mt-1 p-2 bg-gray-50 border border-gray-300 shadow-sm rounded-md focus:ring focus:ring-blue-500 focus:border-blue-500 focus:ring-opacity-50";
            }
            div class="whitespace-nowrap" {
                // TODO: make into a button partial component
                button type="submit" class="py-2 px-4 font-medium rounded-md border border-gray-200" { "Add Feed" }
            }
        }
    }
}
