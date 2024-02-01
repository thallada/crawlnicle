use maud::{html, Markup, PreEscaped};

pub fn opml_import_form() -> Markup {
    html! {
        form
            action="/import/opml"
            method="post"
            id="opml-import-form"
            hx-post="/import/opml"
            hx-encoding="multipart/form-data"
            hx-target="#opml-import-form"
            hx-swap="outerHTML"
            class="flex flex-row gap-6 items-end justify-between"
        {
            div class="grow w-full" {
                label for="opml" class="text-sm font-medium text-gray-700" { "OPML" }
                input
                    type="file"
                    id="opml"
                    name="opml"
                    required="true"
                    accept="text/x-opml,application/xml,text/xml"
                    class="w-full mt-1 p-2 bg-gray-50 border border-gray-300 shadow-sm rounded-md focus:ring focus:ring-blue-500 focus:border-blue-500 focus:ring-opacity-50";
            }
            div class="whitespace-nowrap" {
                button type="submit" class="py-2 px-4 font-medium rounded-md border border-gray-200" { "Import Feeds" }
            }
        }
    }
}
