use maud::{html, Markup};

use crate::models::entry::{Entry, GetEntriesOptions, DEFAULT_ENTRIES_PAGE_SIZE};
use crate::partials::entry_link::{entry_link, EntryLink};

pub fn entry_list(entries: Vec<Entry>, options: &GetEntriesOptions, first_page: bool) -> Markup {
    let len = entries.len() as i64;
    if first_page && len == 0 {
        return html! { p { "No entries found." } };
    }

    let mut more_query = None;
    let limit = options.limit.unwrap_or(DEFAULT_ENTRIES_PAGE_SIZE);
    if len == limit {
        let last_entry = entries.last().unwrap();
        if let Some(feed_id) = options.feed_id {
            more_query = Some(format!(
                "/api/v1/entries?feed_id={}&published_before={}&id_before={}&limit={}",
                feed_id, last_entry.published_at, last_entry.entry_id, limit
            ));
        } else {
            more_query = Some(format!(
                "/api/v1/entries?published_before={}&id_before={}&limit={}",
                last_entry.published_at, last_entry.entry_id, limit
            ));
        }
    }

    html! {
        @for (i, entry) in entries.iter().enumerate() {
            @if i == entries.len() - 1 {
                @if let Some(ref more_query) = more_query {
                    li hx-get=(more_query) hx-trigger="revealed" hx-target="this" hx-swap="afterend" {
                        (EntryLink::new(entry).reset_htmx_target().render())
                        div class="list-loading" {
                            img class="mt-4 max-h-4 invert" src="/static/img/three-dots.svg" alt="Loading...";
                        }
                    }
                } @else {
                    li { (entry_link(entry)) }
                }
            } @else {
                li { (entry_link(entry)) }
            }
        }
    }
}
