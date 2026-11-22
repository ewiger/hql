//! NATS and JetStream, in a build that reaches no server.

use super::reason;
use crate::endpoints::Subject;
use std::time::Duration;

const WHAT: &str = "a `nats` endpoint";

/// One failure, where the native build would have subscribed.
pub(crate) fn subscribed(
    _url: String,
    _subject: String,
    _idle: Option<Duration>,
) -> Box<dyn Iterator<Item = Result<String, String>> + Send> {
    Box::new(std::iter::once(Err(reason(WHAT))))
}

/// One failure, where the native build would have read the stream.
pub(crate) fn messages(
    _url: String,
    _held: Subject,
) -> Box<dyn Iterator<Item = Result<String, String>> + Send> {
    Box::new(std::iter::once(Err(reason(WHAT))))
}

/// A subject a message can be published to: one subject, no wildcard. The
/// rule is the subject's, not the connection's, so it holds in every build.
///
/// # Errors
/// Names the subject when it holds a wildcard.
pub(crate) fn one_subject(subject: &str) -> Result<(), String> {
    if subject.split('.').any(|token| matches!(token, "*" | ">")) {
        return Err(format!(
            "subject `{subject}` holds a wildcard, which reads many subjects and writes none; a \
             message is published to one subject"
        ));
    }
    Ok(())
}

/// A subject open for writing; never constructed in this build.
pub(crate) struct Publisher {
    never: std::convert::Infallible,
}

impl Publisher {
    pub(crate) fn open(_url: &str, subject: &str, _stored: bool) -> Result<Self, String> {
        one_subject(subject)?;
        Err(reason(WHAT))
    }

    pub(crate) fn send(&mut self, _text: String) -> Result<(), String> {
        match self.never {}
    }

    pub(crate) fn flush(&mut self) -> Result<(), String> {
        match self.never {}
    }
}
