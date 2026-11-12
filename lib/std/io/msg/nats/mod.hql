// Provided by the `std` extension.
//
// See doc/proposals/HQL-0047/README.md, doc/proposals/HQL-0048/README.md and
// doc/proposals/HQL-0049/README.md.

/// Core NATS: one subject, as a transport read by `tap` and written by
/// `sink` and `target` like any other endpoint (module `io`). The server
/// keeps nothing: a read is given what is published while it is
/// subscribed, and a message published while no one is subscribed reaches
/// no one. A stream that keeps a subject's messages is
/// `io.msg.nats_jetstream`. The `nats` kind of a configured endpoint is
/// this transport under an id. A program imports the module:
/// `import io.msg.nats`.
module io.msg.nats

import core
import io

/// One core NATS subject on a server. It is on the network and carries
/// messages, each one document, `Json` unless a format is named. It is
/// `Readable` and not `Replayable`: read, it is a `Stream`.
type Nats : {Network, MessageTransport, Readable, Writable}

/// The subject `subject`, as a `Nats` transport; its parameters are the
/// keys of the `nats` kind. A read may name wildcards, `*` for one token
/// and `>` for every token after it; a write names one subject. The
/// server's address is `url`, or the value of the environment variable
/// `url_env` names, one of the two; an address that holds credentials is
/// named by `url_env`. A read ends once no message has arrived for
/// `idle_ms` milliseconds, or never when that is 0. Constructing it opens
/// no connection.
fn nats(subject: String, url: String = "", url_env: String = "", idle_ms: Int = 0) : Nats
