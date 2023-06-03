use axum::response::Response;
use maud::html;

use crate::error::Result;
use crate::partials::layout::Layout;
use crate::log::MEM_LOG;

pub async fn get(layout: Layout) -> Result<Response> {
    let mem_buf = MEM_LOG.lock().unwrap();
    Ok(layout.render(html! {
        pre { (std::str::from_utf8(mem_buf.as_slices().0).unwrap()) }
    }))
}
