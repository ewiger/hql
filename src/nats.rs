//! NATS at the edge: the messages a JetStream stream holds under one
//! subject, read in the stream's order, and messages published to that
//! subject, for the configured endpoints of `kind = "nats"`.
//!
//! The client is asynchronous; each connection owns a runtime of one thread
//! and blocks on it, so the evaluator stays synchronous. A connection is made
//! only when a stream is first traversed or a write begins, never by
//! checking a program.

use crate::endpoints::{Start, Subject};
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
    client: async_nats::Client,
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
            jetstream::new(client.clone())
        };
        Ok(Self {
            context,
            client,
            runtime,
        })
    }
}

/// The messages the stream holds under the subject, each its payload's
/// text, in the stream's order. With `idle`, the messages end once none has
/// arrived for that long; without it they never end.
///
/// Without a consumer the read starts where `start` says and leaves nothing
/// on the server. With one, the server keeps the position under that name:
/// the read starts after the last message acknowledged, and `start` is where
/// the consumer's first read began.
pub(crate) fn messages(
    url: String,
    held: Subject,
) -> Box<dyn Iterator<Item = Result<String, String>> + Send> {
    let mut reader: Option<Reader> = None;
    let mut ended = false;
    Box::new(std::iter::from_fn(move || {
        if ended {
            return None;
        }
        let next = match reader.as_mut() {
            Some(open) => open.next(),
            None => match Reader::open(&url, &held) {
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

/// The messages of an ordered consumer, a failure as its text.
type Arriving =
    std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<jetstream::Message, String>> + Send>>;

/// How many messages a durable consumer is asked for at once.
const PULL: usize = 100;

/// How long one request to a durable consumer waits for a message.
const WAIT: Duration = Duration::from_secs(1);

/// How a stream's messages arrive.
enum Feed {
    /// Through an ordered consumer, which holds no state on the server
    /// beyond the read.
    Ordered(Arriving),
    /// Through a durable consumer, the server's record of a position. The
    /// messages asked for and not yet given are held here, so a read that
    /// stops hands each one back.
    Durable {
        consumer: Box<consumer::Consumer<consumer::pull::Config>>,
        ready: std::collections::VecDeque<jetstream::Message>,
        /// The message given last, acknowledged once the program asks for
        /// the one after it.
        given: Option<Box<jetstream::Message>>,
    },
}

/// A stream being read through a consumer.
struct Reader {
    session: Session,
    stream: String,
    feed: Option<Feed>,
    idle: Option<Duration>,
}

impl Drop for Reader {
    fn drop(&mut self) {
        // The consumer ends on the runtime it was made on. Every message
        // held and not acknowledged is handed back, so the next read under
        // the consumer's name starts with it and does not wait for the
        // server to give up on this one.
        let feed = self.feed.take();
        self.session.runtime.block_on(async {
            if let Some(Feed::Durable { ready, given, .. }) = feed {
                for message in given.map(|given| *given).into_iter().chain(ready) {
                    let _ = message.ack_with(jetstream::AckKind::Nak(None)).await;
                }
                // Sent before the connection closes.
                let _ = self.session.client.flush().await;
            }
        });
    }
}

impl Reader {
    fn open(url: &str, held: &Subject) -> Result<Self, String> {
        let session = Session::open(url)?;
        let (stream, subject) = (&held.stream, &held.subject);
        let deliver_policy = match held.start {
            Start::Earliest => consumer::DeliverPolicy::All,
            Start::Latest => consumer::DeliverPolicy::New,
        };
        let located = |error: String| format!("stream `{stream}`, subject `{subject}`: {error}");
        let feed = session.runtime.block_on(async {
            let on = session
                .context
                .get_stream(stream)
                .await
                .map_err(|error| format!("stream `{stream}`: {error}"))?;
            let Some(name) = &held.consumer else {
                let messages = on
                    .create_consumer(consumer::pull::OrderedConfig {
                        filter_subject: subject.clone(),
                        deliver_policy,
                        ..Default::default()
                    })
                    .await
                    .map_err(|error| located(error.to_string()))?
                    .messages()
                    .await
                    .map_err(|error| located(error.to_string()))?;
                return Ok(Feed::Ordered(Box::pin(
                    messages.map(|message| message.map_err(|error| error.to_string())),
                )));
            };
            // Made by the first read under its name, and found by every
            // read after it.
            let mut consumer = on
                .get_or_create_consumer(
                    name,
                    consumer::pull::Config {
                        durable_name: Some(name.clone()),
                        filter_subject: subject.clone(),
                        deliver_policy,
                        ack_policy: consumer::AckPolicy::Explicit,
                        ..Default::default()
                    },
                )
                .await
                .map_err(|error| located(error.to_string()))?;
            let reads = consumer
                .info()
                .await
                .map_err(|error| located(error.to_string()))?
                .config
                .filter_subject
                .clone();
            if reads != *subject {
                return Err(format!(
                    "stream `{stream}`: the consumer `{name}` reads the subject `{reads}`, not \
                     `{subject}`; a consumer keeps one position, over one subject"
                ));
            }
            Ok(Feed::Durable {
                consumer: Box::new(consumer),
                ready: std::collections::VecDeque::new(),
                given: None,
            })
        })?;
        Ok(Self {
            session,
            stream: stream.clone(),
            feed: Some(feed),
            idle: held.idle,
        })
    }

    /// The next message's payload, or `None` once the reader has been idle
    /// too long. Asking for it acknowledges the message given before.
    fn next(&mut self) -> Option<Result<String, String>> {
        let stream = &self.stream;
        let idle = self.idle;
        let arrived = self.session.runtime.block_on(async {
            match self.feed.as_mut()? {
                Feed::Ordered(messages) => match idle {
                    Some(idle) => tokio::time::timeout(idle, messages.next())
                        .await
                        .unwrap_or(None),
                    None => messages.next().await,
                },
                Feed::Durable {
                    consumer,
                    ready,
                    given,
                } => {
                    if let Some(given) = given.take()
                        && let Err(error) = given.ack().await
                    {
                        return Some(Err(format!("a message was not acknowledged: {error}")));
                    }
                    let since = std::time::Instant::now();
                    while ready.is_empty() {
                        let waited = since.elapsed();
                        if idle.is_some_and(|idle| waited >= idle) {
                            return None;
                        }
                        // What the stream holds now, at once; and where it
                        // holds nothing, the next message to arrive.
                        let wait = idle.map_or(WAIT, |idle| (idle - waited).min(WAIT));
                        let pulled = match consumer.fetch().max_messages(PULL).messages().await {
                            Ok(batch) => batch.collect::<Vec<_>>().await,
                            Err(error) => return Some(Err(error.to_string())),
                        };
                        let pulled = if pulled.is_empty() {
                            match consumer
                                .batch()
                                .max_messages(1)
                                .expires(wait)
                                .messages()
                                .await
                            {
                                Ok(batch) => batch.collect::<Vec<_>>().await,
                                Err(error) => return Some(Err(error.to_string())),
                            }
                        } else {
                            pulled
                        };
                        for message in pulled {
                            match message {
                                Ok(message) => ready.push_back(message),
                                Err(error) => return Some(Err(error.to_string())),
                            }
                        }
                    }
                    let message = ready.pop_front()?;
                    // Held to be acknowledged; the copy is what is decoded.
                    let copy = jetstream::Message {
                        message: message.message.clone(),
                        context: message.context.clone(),
                    };
                    *given = Some(Box::new(message));
                    Some(Ok(copy))
                }
            }
        })?;
        Some(match arrived {
            Ok(message) => String::from_utf8(message.payload.to_vec()).map_err(|_| {
                let sequence = message
                    .info()
                    .map(|info| info.stream_sequence.to_string())
                    .unwrap_or_else(|_| "?".to_owned());
                format!("stream `{stream}`, sequence {sequence}: the message is not UTF-8 text")
            }),
            Err(error) => Err(format!("stream `{stream}`: {error}")),
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
