use maud::{html, Markup};

use crate::models::entry::Entry;
use crate::partials::entry_link::entry_link;

pub fn entry_list(entries: Vec<Entry>) -> Markup {
    html! {
        ul class="entries" {
            @for entry in entries {
                li class="entry" { (entry_link(entry)) }
            }
        }
    }
}
