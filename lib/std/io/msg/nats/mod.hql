// Provided by the `std` extension.
//
// See doc/proposals/HQL-0047/README.md, doc/proposals/HQL-0048/README.md and
// doc/proposals/HQL-0049/README.md.

/// NATS JetStream: one subject of a stream, as a transport read by `tap`
/// and written by `sink` and `target` like any other endpoint (module
/// `io`). The `nats` kind of a configured endpoint is this transport under
/// an id. A program imports the module: `import io.msg.nats`.
module io.msg.nats

import core
import io

/// One subject of a JetStream stream on a NATS server. It is on the
/// network and carries messages, each one document, `Json` unless a format
/// is named. A stream is a log that does not end, so it is `Readable` and
/// not `Replayable`: read, it is a `Stream`.
type Nats : {Network, MessageTransport, Readable, Writable}

/// The subject `subject` of the stream `stream`, as a `Nats` transport; its
/// parameters are the keys of the `nats` kind. The server's address is
/// `url`, or the value of the environment variable `url_env` names, one of
/// the two; an address that holds credentials is named by `url_env`. A read
/// starts at the `"earliest"` message the stream keeps, or at the next one
/// published under `from = "latest"`, and ends once no message has arrived
/// for `idle_ms` milliseconds, or never when that is 0. With `consumer`,
/// the server keeps the read's position under that name, so a later read
/// starts after the last message acknowledged. Constructing it opens no
/// connection.
fn nats(subject: String, stream: String, url: String = "", url_env: String = "", from: String = "earliest", idle_ms: Int = 0, consumer: String = "") : Nats
