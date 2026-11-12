//! NATS at the edge: the messages a JetStream stream holds under one
//! subject, read in the stream's order, and messages published to that
//! subject, for the configured endpoints of `kind = "nats"`.
//!
//! The client is asynchronous; each connection owns a runtime of one thread
//! and blocks on it, so the evaluator stays synchronous. A connection is made
//! only when a stream is first traversed or a write begins, never by
//! checking a program.

use crate::endpoints::Start;
use async_nats::jetstream::{self, consumer};
use futures_util::StreamExt;
use std::time::Duration;

/// How long a connection may take before it is a failure.
const DEADLINE: Duration = Duration::from_secs(10);

/// How many messages are published before their acknowledgements are awaited.
const BATCH: usize = 100;

/// A connection to a server, with the runtime its client runs on.
struct Session {
    // Dropped before the runtime it was made on.
    context: jetstream::Context,
    runtime: tokio::runtime::Runtime,
}

impl Session {
    /// # Errors
    /// Says that no server answered; the URL is never repeated, since it may
    /// hold a password.
    fn open(url: &str) -> Result<Self, String> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| format!("no runtime for the NATS client: {error}"))?;
        let client = runtime
            .block_on(
                async_nats::ConnectOptions::new()
                    .name("hql")
                    .connection_timeout(DEADLINE)
                    .connect(url),
            )
            .map_err(|error| format!("no NATS server answered: {error}"))?;
        // The context is made, as it is used, on the runtime.
        let context = {
            let _on = runtime.enter();
            jetstream::new(client)
        };
        Ok(Self { context, runtime })
    }
}

/// The messages the stream holds under `subject`, each its payload's text,
/// in the stream's order from where `start` says. With `idle`, the messages
/// end once none has arrived for that long; without it they never end.
pub(crate) fn messages(
    url: String,
    stream: String,
    subject: String,
    start: Start,
    idle: Option<Duration>,
) -> Box<dyn Iterator<Item = Result<String, String>> + Send> {
    let mut reader: Option<Reader> = None;
    let mut ended = false;
    Box::new(std::iter::from_fn(move || {
        if ended {
            return None;
        }
        let next = match reader.as_mut() {
            Some(open) => open.next(),
            None => match Reader::open(&url, &stream, &subject, start, idle) {
                Ok(open) => reader.insert(open).next(),
                Err(error) => Some(Err(error)),
            },
        };
        if !matches!(next, Some(Ok(_))) {
            ended = true;
        }
        next
    }))
}

/// A stream being read through an ordered consumer, which holds no state on
/// the server beyond the read.
struct Reader {
    session: Session,
    stream: String,
    messages: Option<consumer::pull::Ordered>,
    idle: Option<Duration>,
}

impl Drop for Reader {
    fn drop(&mut self) {
        // The consumer ends on the runtime it was made on.
        let _on = self.session.runtime.enter();
        self.messages.take();
    }
}

impl Reader {
    fn open(
        url: &str,
        stream: &str,
        subject: &str,
        start: Start,
        idle: Option<Duration>,
    ) -> Result<Self, String> {
        let session = Session::open(url)?;
        let messages = session.runtime.block_on(async {
            let held = session
                .context
                .get_stream(stream)
                .await
                .map_err(|error| format!("stream `{stream}`: {error}"))?;
            let consumer = held
                .create_consumer(consumer::pull::OrderedConfig {
                    filter_subject: subject.to_owned(),
                    deliver_policy: match start {
                        Start::Earliest => consumer::DeliverPolicy::All,
                        Start::Latest => consumer::DeliverPolicy::New,
                    },
                    ..Default::default()
                })
                .await
                .map_err(|error| format!("stream `{stream}`, subject `{subject}`: {error}"))?;
            consumer
                .messages()
                .await
                .map_err(|error| format!("stream `{stream}`, subject `{subject}`: {error}"))
        })?;
        Ok(Self {
            session,
            stream: stream.to_owned(),
            messages: Some(messages),
            idle,
        })
    }

    /// The next message's payload, or `None` once the reader has been idle
    /// too long.
    fn next(&mut self) -> Option<Result<String, String>> {
        let messages = self.messages.as_mut()?;
        let arrived = self.session.runtime.block_on(async {
            match self.idle {
                Some(idle) => tokio::time::timeout(idle, messages.next())
                    .await
                    .unwrap_or(None),
                None => messages.next().await,
            }
        })?;
        Some(match arrived {
            Ok(message) => String::from_utf8(message.payload.to_vec()).map_err(|_| {
                let sequence = message
                    .info()
                    .map(|info| info.stream_sequence.to_string())
                    .unwrap_or_else(|_| "?".to_owned());
                format!(
                    "stream `{}`, sequence {sequence}: the message is not UTF-8 text",
                    self.stream
                )
            }),
            Err(error) => Err(format!("stream `{}`: {error}", self.stream)),
        })
    }
}

/// A subject open for publishing: a message is written once a stream that
/// holds the subject acknowledges it.
pub(crate) struct Publisher {
    session: Session,
    subject: String,
    pending: Vec<jetstream::context::PublishAckFuture>,
}

impl Publisher {
    /// Connect to the server.
    ///
    /// # Errors
    /// Says that no server answered, or that the subject holds a wildcard:
    /// a message is published to one subject.
    pub(crate) fn open(url: &str, subject: &str) -> Result<Self, String> {
        if subject.split('.').any(|token| matches!(token, "*" | ">")) {
            return Err(format!(
                "subject `{subject}` holds a wildcard, which reads many subjects and writes \
                 none; a message is published to one subject"
            ));
        }
        Ok(Self {
            session: Session::open(url)?,
            subject: subject.to_owned(),
            pending: Vec::new(),
        })
    }

    /// Publish one message whose payload is `text`, awaiting the
    /// acknowledgements once a batch is out.
    ///
    /// # Errors
    /// Names the subject when a message is not sent or not acknowledged.
    pub(crate) fn send(&mut self, text: String) -> Result<(), String> {
        let acknowledged = self
            .session
            .runtime
            .block_on(
                self.session
                    .context
                    .publish(self.subject.clone(), text.into()),
            )
            .map_err(|error| format!("subject `{}`: {error}", self.subject))?;
        self.pending.push(acknowledged);
        if self.pending.len() >= BATCH {
            self.flush()?;
        }
        Ok(())
    }

    /// Await the acknowledgement of every message published.
    ///
    /// # Errors
    /// Names the subject when a stream did not store a message: no stream
    /// holds the subject, or the stream refused it.
    pub(crate) fn flush(&mut self) -> Result<(), String> {
        let pending = std::mem::take(&mut self.pending);
        self.session.runtime.block_on(async {
            for acknowledged in pending {
                acknowledged.await.map_err(|error| {
                    format!(
                        "subject `{}`: no stream stored the message: {error}",
                        self.subject
                    )
                })?;
            }
            Ok(())
        })
    }
}
