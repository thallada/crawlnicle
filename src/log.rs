use std::sync::Mutex;
use std::{collections::VecDeque, io::Write};

use anyhow::Result;
use bytes::Bytes;
use once_cell::sync::Lazy;
use tokio::sync::watch::Sender;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

use crate::config::Config;

/// A shared in-memory buffer to store log bytes
pub static MEM_LOG: Lazy<Mutex<VecDeque<u8>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

/// A `Writer` to a shared static in-memory buffer that stores bytes up until `max` bytes, at which
/// point it will truncate the buffer from the front up to the first newline byte `\n` within the
/// size limit.
///
/// This is useful for storing the last emitted log lines of an application in-memory without
/// needing to worry about the memory growing infinitely large.
///
/// `LimitedInMemoryBuffer` does not guarantee that the memory usage is less than `max`.
/// VecDeque`'s capacity may exceed `max` and it will only check and truncate the size of the
/// internal buffer *before* writing to it. It will continue to write, even if the size of the line
/// to write will make the buffer exceed `max`.
struct LimitedInMemoryBuffer {
    pub buf: &'static Mutex<VecDeque<u8>>,
    sender: Sender<Bytes>,
    max: usize,
}

impl LimitedInMemoryBuffer {
    fn new(buf: &'static Mutex<VecDeque<u8>>, sender: Sender<Bytes>, max: usize) -> Self {
        Self { buf, sender, max }
    }

    /// Truncate the buffer to max bytes plus bytes before the closest newline starting from the
    /// front
    fn truncate(&mut self) {
        let mut buf = self.buf.lock().unwrap();
        let len = buf.len();
        if len > self.max {
            drop(buf.drain(..len - self.max));
            let mut i = 0;
            while i < buf.len() {
                if buf[i] == b'\n' {
                    break;
                }
                i += 1;
            }
            drop(buf.drain(..i));
        }
    }
}

impl Write for LimitedInMemoryBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // if self.buf is too big, truncate it to the closest newline starting from the front
        self.truncate();
        let bytes = Bytes::copy_from_slice(buf);
        self.sender.send(bytes).ok();
        let mut mem_buf = self.buf.lock().unwrap();
        mem_buf.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut buf = self.buf.lock().unwrap();
        buf.flush()
    }
}

pub fn init_tracing(
    config: &Config,
    log_sender: Sender<Bytes>,
) -> Result<(WorkerGuard, WorkerGuard)> {
    let fmt_layer = tracing_subscriber::fmt::layer();
    let filter_layer = EnvFilter::from_default_env();
    let file_appender = tracing_appender::rolling::hourly("./logs", "log");
    let (file_writer, file_writer_guard) = tracing_appender::non_blocking(file_appender);
    let mem_writer = LimitedInMemoryBuffer::new(&MEM_LOG, log_sender, config.max_mem_log_size);
    let (mem_writer, mem_writer_guard) = tracing_appender::non_blocking(mem_writer);
    let file_writer_layer = tracing_subscriber::fmt::layer().with_writer(file_writer);
    let mem_writer_layer = tracing_subscriber::fmt::layer().with_writer(mem_writer);
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(file_writer_layer)
        .with(mem_writer_layer)
        .init();
    Ok((file_writer_guard, mem_writer_guard))
}
