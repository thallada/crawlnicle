use apalis::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum AsyncJob {
    HelloWorld(String),
}

impl Job for AsyncJob {
    const NAME: &'static str = "apalis::AsyncJob";
}
