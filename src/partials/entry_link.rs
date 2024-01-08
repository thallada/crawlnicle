use maud::{html, Markup};

use crate::models::entry::Entry;
use crate::partials::link::{link, LinkProps};
use crate::utils::get_domain;
use crate::uuid::Base62Uuid;

pub struct EntryLink<'a> {
    pub entry: &'a Entry,
    pub reset_htmx_target: bool,
}

impl EntryLink<'_> {
    pub fn new(entry: &Entry) -> EntryLink {
        EntryLink {
            entry,
            reset_htmx_target: false,
        }
    }

    pub fn reset_htmx_target(&mut self) -> &mut Self {
        self.reset_htmx_target = true;
        self
    }

    pub fn render(&self) -> Markup {
        let title = self
            .entry
            .title
            .as_ref()
            .cloned()
            .unwrap_or_else(|| "Untitled".to_string());
        let url = format!("/entry/{}", Base62Uuid::from(self.entry.entry_id));
        let domain = get_domain(&self.entry.url).unwrap_or_default();
        html! {
            div class="flex flex-row gap-4" {
                (link(LinkProps { destination: &url, title: &title, reset_htmx_target: self.reset_htmx_target }))
                em class="text-gray-600" { (domain) }
            }
        }
    }
}

pub fn entry_link(entry: &Entry) -> Markup {
    EntryLink::new(entry).render()
}
