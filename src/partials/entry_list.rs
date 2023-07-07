use maud::{html, Markup};

use crate::models::entry::Entry;
use crate::utils::get_domain;
use crate::uuid::Base62Uuid;

pub fn entry_list(entries: Vec<Entry>) -> Markup {
    html! {
        ul class="entries" {
            @for entry in entries {
                @let title = entry.title.unwrap_or_else(|| "Untitled".to_string());
                @let url = format!("/entry/{}", Base62Uuid::from(entry.entry_id));
                @let domain = get_domain(&entry.url).unwrap_or_default();
                li { a href=(url) { (title) } em class="domain" { (domain) }}
            }
        }
    }
}
