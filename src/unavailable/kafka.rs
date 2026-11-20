//! Kafka, in a build that reaches no server.

use super::reason;
use std::time::Duration;

pub use crate::endpoints::Start;

const WHAT: &str = "a `kafka` endpoint";

/// One failure, where the native build would have connected.
pub(crate) fn messages(
    _brokers: Vec<String>,
    _topic: String,
    _start: Start,
    _idle: Option<Duration>,
) -> Box<dyn Iterator<Item = Result<String, String>> + Send> {
    Box::new(std::iter::once(Err(reason(WHAT))))
}

/// A topic open for writing; never constructed in this build.
pub(crate) struct Producer {
    never: std::convert::Infallible,
}

impl Producer {
    pub(crate) fn open(_brokers: &[String], _topic: &str) -> Result<Self, String> {
        Err(reason(WHAT))
    }

    pub(crate) fn send(&mut self, _text: String) -> Result<(), String> {
        match self.never {}
    }

    pub(crate) fn flush(&mut self) -> Result<(), String> {
        match self.never {}
    }
}
