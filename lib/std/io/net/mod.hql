// Provided by the `std` extension.
//
// See doc/proposals/HQL-0021/README.md.

/// Network endpoints: transports, read and written by `source`, `tap`,
/// `target` and `sink` like any other endpoint (module `io`). Constructing
/// one sends nothing: the request a program describes happens when a
/// source, tap, target or sink uses it, and only if the host grants the
/// network to that program. A document reaches no network. A program
/// imports the module: `import net`.
module io.net

import core
import io

/// An HTTP resource at `url`. Read, it is a `GET` whose response body is
/// its bytes; the host reads it once per evaluation, so it is
/// `Replayable`. Written, it is a request by `method` whose body is the
/// document, and the response is not read beyond its status. Both send
/// `headers`, a map of header names to values. Authentication belongs in a
/// configured source, not in the program.
type Http : {Replayable, Writable} {
    url     : String
    method  : String   // "GET" unless written
    headers : Data     // a map of header names to values
}

/// A WebSocket connection to `url`: each message is a document, read as it
/// arrives and written as it is sent.
type WebSocket : {Readable, Writable} {
    url : String
}

/// A TCP connection to `host` on `port`. A socket carries bytes, not
/// messages, so the documents on it are separated as their format
/// separates them: whitespace between `Json` values, a line for `Lines`,
/// `JsonLines` and `Csv`, a `---` line for `Yaml`.
type Socket : {Readable, Writable} {
    host : String
    port : Int
}

/// The HTTP resource at `url`, as an `Http` endpoint: `method` is the
/// request a write sends, `"GET"` unless given, and `headers` a map of
/// header names to values sent with every request. Constructing it sends
/// nothing.
fn http(url: String, method: String = "GET", headers: Data = {:}) : Http

/// The WebSocket at `url`, as a `WebSocket` endpoint. Constructing it opens
/// no connection.
fn websocket(url: String) : WebSocket

/// The TCP endpoint `host` on `port`, as a `Socket`. Constructing it opens
/// no connection.
fn socket(host: String, port: Int) : Socket
