// Network endpoints: transports, read and written by source, tap, target and
// sink like any other endpoint (io/init.hql). Provided by the `net` extension,
// which a program imports: `import net`.
//
// See doc/proposals/HQL-0021/README.md.
//
// Constructing one sends nothing. The request a program describes happens
// when a source, tap, target or sink uses it, and only if the host grants the
// network to that program.

// An HTTP resource. Read, it is a GET whose response body is its bytes; the
// host reads it once per evaluation, so it is Replayable. Written, it is a
// request whose body is the document, by `method`, and the response is not
// read. Authentication belongs in a configured source, not in the program.
type Http : {Replayable, Writable} {
    url     : String
    method  : String   // "GET" unless written
    headers : Data     // a map of header names to values
}

// A WebSocket connection: each message is a document, read as it arrives
// and written as it is sent.
type WebSocket : {Readable, Writable} {
    url : String
}

// A TCP connection. A socket carries bytes, not messages, so the documents on
// it are separated as their format separates them: whitespace between Json
// values, a line for Text and Csv, a `---` line for Yaml.
type Socket : {Readable, Writable} {
    host : String
    port : Int
}

// Constructors, provided by the `net` extension; each is Pure:
// http(url: String, method: String = "GET", headers: Data = {}) : Http
// websocket(url: String) : WebSocket
// socket(host: String, port: Int) : Socket
