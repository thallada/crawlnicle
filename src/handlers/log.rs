use std::convert::Infallible;
use std::time::Duration;

use axum::extract::State;
use axum::response::{
    sse::{Event, Sse},
    Response,
};
use bytes::Bytes;
use maud::html;
use tokio::sync::watch::Receiver;
use tokio_stream::wrappers::WatchStream;
use tokio_stream::Stream;
use tokio_stream::StreamExt;

use crate::error::Result;
use crate::log::MEM_LOG;
use crate::partials::layout::Layout;

pub async fn get(layout: Layout) -> Result<Response> {
    let mem_buf = MEM_LOG.lock().unwrap();
    Ok(layout.render(html! {
        turbo-stream-source src="/log/stream" {}
        pre id="log" { (std::str::from_utf8(mem_buf.as_slices().0).unwrap()) }
    }))
}

pub async fn stream(
    State(log_receiver): State<Receiver<Bytes>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let log_stream = WatchStream::new(log_receiver);
    let log_stream = log_stream.map(|line| {
        Ok(Event::default().data(html! {
            turbo-stream action="append" target="log" {
                template {
                    (std::str::from_utf8(&line).unwrap())
                }
            }
        }.into_string()))
    });
    Sse::new(log_stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive-text"),
    )
}
