use maud::{html, Markup};

use crate::models::entry::Entry;
use crate::utils::get_domain;
use crate::uuid::Base62Uuid;

pub fn entry_link(entry: &Entry) -> Markup {
    let title = entry.title.as_ref().map(|s| s.clone()).unwrap_or_else(|| "Untitled".to_string());
    let url = format!("/entry/{}", Base62Uuid::from(entry.entry_id));
    let domain = get_domain(&entry.url).unwrap_or_default();
    html! {
        a href=(url) class="entry-link" { (title) } em class="entry-link-domain" { (domain) }
    }
}
