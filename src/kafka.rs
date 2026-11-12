//! Kafka at the edge: the messages of one topic, read in order from each of
//! its partitions, and messages written to it, for the configured endpoints of
//! `kind = "kafka"`.
//!
//! The client is asynchronous; each connection owns a runtime of one thread
//! and blocks on it, so the evaluator stays synchronous. A connection is made
//! only when a stream is first traversed or a write begins, never by
//! checking a program.

use rskafka::client::partition::{Compression, OffsetAt, PartitionClient, UnknownTopicHandling};
use rskafka::client::{Client, ClientBuilder};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// How long a connection, or one request, may keep retrying before it is a
/// failure.
const DEADLINE: Duration = Duration::from_secs(10);

/// How long one fetch waits at the broker for a message to arrive.
const WAIT_MS: i32 = 250;

/// Where reading a topic starts in each partition.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Start {
    /// The earliest message the topic still keeps.
    #[default]
    Earliest,
    /// The first message written after the read begins.
    Latest,
}

impl Start {
    /// The word `from =` writes it as.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Earliest => "earliest",
            Self::Latest => "latest",
        }
    }
}

/// A connection to a cluster, with the runtime its client runs on.
struct Session {
    runtime: tokio::runtime::Runtime,
    client: Client,
}

impl Session {
    fn open(brokers: &[String]) -> Result<Self, String> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| format!("no runtime for the Kafka client: {error}"))?;
        let backoff = rskafka::BackoffConfig {
            deadline: Some(DEADLINE),
            ..rskafka::BackoffConfig::default()
        };
        let client = runtime
            .block_on(
                ClientBuilder::new(brokers.to_vec())
                    .client_id("hql")
                    .backoff_config(backoff)
                    .build(),
            )
            .map_err(|error| format!("no broker at {} answered: {error}", brokers.join(",")))?;
        Ok(Self { runtime, client })
    }

    /// The partitions of `topic`, in order.
    fn partitions(&self, topic: &str) -> Result<Vec<i32>, String> {
        let topics = self
            .runtime
            .block_on(self.client.list_topics())
            .map_err(|error| format!("topic `{topic}`: {error}"))?;
        topics
            .into_iter()
            .find(|held| held.name == topic)
            .map(|held| held.partitions.into_iter().collect())
            .ok_or_else(|| format!("the cluster holds no topic `{topic}`"))
    }

    fn partition(&self, topic: &str, partition: i32) -> Result<PartitionClient, String> {
        self.runtime
            .block_on(
                self.client
                    .partition_client(topic, partition, UnknownTopicHandling::Error),
            )
            .map_err(|error| format!("topic `{topic}`, partition {partition}: {error}"))
    }
}

/// The messages of a topic, each its value's text, read from every
/// partition in turn from where `start` says. With `idle`, the messages end
/// once none has arrived for that long; without it they never end.
pub(crate) fn messages(
    brokers: Vec<String>,
    topic: String,
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
            None => match Reader::open(&brokers, &topic, start, idle) {
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

/// A topic being read: a client and the next offset for each partition.
struct Reader {
    session: Session,
    topic: String,
    partitions: Vec<(PartitionClient, i64)>,
    ready: VecDeque<String>,
    idle: Option<Duration>,
    last: Instant,
}

impl Reader {
    fn open(
        brokers: &[String],
        topic: &str,
        start: Start,
        idle: Option<Duration>,
    ) -> Result<Self, String> {
        let session = Session::open(brokers)?;
        let at = match start {
            Start::Earliest => OffsetAt::Earliest,
            Start::Latest => OffsetAt::Latest,
        };
        let mut partitions = Vec::new();
        for partition in session.partitions(topic)? {
            let client = session.partition(topic, partition)?;
            let offset = session
                .runtime
                .block_on(client.get_offset(at))
                .map_err(|error| format!("topic `{topic}`, partition {partition}: {error}"))?;
            partitions.push((client, offset));
        }
        Ok(Self {
            session,
            topic: topic.to_owned(),
            partitions,
            ready: VecDeque::new(),
            idle,
            last: Instant::now(),
        })
    }

    /// The next message's value, fetching from each partition in turn
    /// until one arrives, or `None` once the reader has been idle too long.
    fn next(&mut self) -> Option<Result<String, String>> {
        loop {
            if let Some(text) = self.ready.pop_front() {
                self.last = Instant::now();
                return Some(Ok(text));
            }
            if self.idle.is_some_and(|idle| self.last.elapsed() >= idle) {
                return None;
            }
            if self.partitions.is_empty() {
                // A topic with no partitions holds nothing to wait for.
                return None;
            }
            for (client, offset) in &mut self.partitions {
                let fetched = self.session.runtime.block_on(client.fetch_records(
                    *offset,
                    1..1_000_000,
                    WAIT_MS,
                ));
                let (records, _) = match fetched {
                    Ok(fetched) => fetched,
                    Err(error) => {
                        return Some(Err(format!(
                            "topic `{}`, partition {}: {error}",
                            self.topic,
                            client.partition()
                        )));
                    }
                };
                for record in records {
                    *offset = record.offset + 1;
                    // A message without a value, a tombstone, holds no
                    // document.
                    let Some(value) = record.record.value else {
                        continue;
                    };
                    match String::from_utf8(value) {
                        Ok(text) => self.ready.push_back(text),
                        Err(_) => {
                            return Some(Err(format!(
                                "topic `{}`, partition {}, offset {}: the message is not UTF-8 \
                                 text",
                                self.topic,
                                client.partition(),
                                record.offset
                            )));
                        }
                    }
                }
            }
        }
    }
}

/// A topic open for writing: each message goes to its first partition, so
/// the messages one program writes keep their order.
pub(crate) struct Producer {
    session: Session,
    client: PartitionClient,
    topic: String,
    pending: Vec<rskafka::record::Record>,
}

/// How many messages are sent together.
const BATCH: usize = 100;

impl Producer {
    /// Connect to the cluster and the topic's first partition.
    ///
    /// # Errors
    /// Names the brokers or the topic when either cannot be reached.
    pub(crate) fn open(brokers: &[String], topic: &str) -> Result<Self, String> {
        let session = Session::open(brokers)?;
        let first = session
            .partitions(topic)?
            .into_iter()
            .min()
            .ok_or_else(|| format!("topic `{topic}` has no partition to write to"))?;
        let client = session.partition(topic, first)?;
        Ok(Self {
            session,
            client,
            topic: topic.to_owned(),
            pending: Vec::new(),
        })
    }

    /// Queue one message whose value is `text`, sending the queue once it is
    /// a batch.
    ///
    /// # Errors
    /// Names the topic when a batch is not accepted.
    pub(crate) fn send(&mut self, text: String) -> Result<(), String> {
        self.pending.push(rskafka::record::Record {
            key: None,
            value: Some(text.into_bytes()),
            headers: std::collections::BTreeMap::new(),
            timestamp: chrono::Utc::now(),
        });
        if self.pending.len() >= BATCH {
            self.flush()?;
        }
        Ok(())
    }

    /// Send every queued message.
    ///
    /// # Errors
    /// Names the topic when the broker does not accept them.
    pub(crate) fn flush(&mut self) -> Result<(), String> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let records = std::mem::take(&mut self.pending);
        self.session
            .runtime
            .block_on(self.client.produce(records, Compression::NoCompression))
            .map(drop)
            .map_err(|error| format!("topic `{}`: {error}", self.topic))
    }
}
