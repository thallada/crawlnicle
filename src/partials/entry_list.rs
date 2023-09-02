use maud::{html, Markup};

use crate::models::entry::{Entry, GetEntriesOptions, DEFAULT_ENTRIES_PAGE_SIZE};
use crate::partials::entry_link::entry_link;

pub fn entry_list(entries: Vec<Entry>, options: &GetEntriesOptions) -> Markup {
    let len = entries.len() as i64;
    if len == 0 {
        return html! { p { "No entries found." } };
    }

    let mut more_query = None;
    if len == options.limit.unwrap_or(DEFAULT_ENTRIES_PAGE_SIZE) {
        let last_entry = entries.last().unwrap();
        if let Some(feed_id) = options.feed_id {
            more_query = Some(format!(
                "/api/v1/entries?feed_id={}&published_before={}&id_before={}",
                feed_id,
                last_entry.published_at,
                last_entry.entry_id
            ));
        } else {
            more_query = Some(format!(
                "/api/v1/entries?published_before={}&id_before={}",
                last_entry.published_at,
                last_entry.entry_id
            ));
        }
    }

    html! {
        @for (i, entry) in entries.iter().enumerate() {
            @if i == entries.len() - 1 {
                @if let Some(ref more_query) = more_query {
                    li class="entry" hx-get=(more_query) hx-trigger="revealed" hx-swap="afterend" {
                        (entry_link(entry))
                        div class="htmx-indicator list-loading" {
                            img class="loading" src="/static/img/three-dots.svg" alt="Loading...";
                        }
                    }
                } @else {
                    li class="entry" { (entry_link(entry)) }
                }
            } @else {
                li class="entry" { (entry_link(entry)) }
            }
        }
    }
}
