use std::convert::Infallible;
use std::str::from_utf8;
use std::time::Duration;

use ansi_to_html::convert;
use axum::extract::State;
use axum::response::sse::KeepAlive;
use axum::response::{
    sse::{Event, Sse},
    Response,
};
use axum_extra::TypedHeader;
use bytes::Bytes;
use maud::{html, PreEscaped};
use tokio::sync::watch::Receiver;
use tokio_stream::wrappers::WatchStream;
use tokio_stream::Stream;
use tokio_stream::StreamExt;

use crate::error::Result;
use crate::htmx::HXTarget;
use crate::log::MEM_LOG;
use crate::partials::layout::Layout;

pub async fn get(hx_target: Option<TypedHeader<HXTarget>>, layout: Layout) -> Result<Response> {
    let mem_buf = MEM_LOG.lock().unwrap();
    Ok(layout
        .with_subtitle("log")
        .targeted(hx_target)
        .render(html! {
            pre id="log" class="text-sm" hx-sse="connect:/log/stream swap:message" hx-swap="beforeend" hx-target="#log" {
                (PreEscaped(convert(from_utf8(mem_buf.as_slices().0).unwrap()).unwrap()))
            }
        }))
}

pub async fn stream(
    State(log_receiver): State<Receiver<Bytes>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let log_stream = WatchStream::new(log_receiver);
    let log_stream = log_stream.map(|line| {
        Ok(Event::default().data(
            html! {
                (PreEscaped(convert(from_utf8(&line).unwrap()).unwrap()))
            }
            .into_string(),
        ))
    });
    Sse::new(log_stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive-text"),
    )
}
