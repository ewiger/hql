# HQL-0047: Transports

**Status**: implemented in part
**Created**: 2026-11-12
**Source**: [HQL-0021](../HQL-0021/README.md) (endpoints, `source`, `tap`,
`target`, `sink`, the network and what a host grants),
[HQL-0012](../HQL-0012/README.md) (configured endpoints and source kinds),
[HQL-0035](../HQL-0035/README.md) (the schema boundary),
[HQL-0046](../HQL-0046/README.md) (Kafka topics and PostgreSQL tables),
`lib/std/io/mod.hql`, `lib/std/io/net/mod.hql`, `src/endpoints.rs`, `src/io.rs`,
`src/net.rs`, `src/extensions/net.rs`

## Abstract

A **transport** is an endpoint that reaches outside the evaluation: a file, a
process stream, a network resource, a database table or a broker topic.
`Transport` is an abstract subtype of `Endpoint`, and two further supertypes
state what the checker and the host need of one: its **locality**, `Direct` or
`Network`, and its **carriage**, bytes, messages or rows. A host policy grants
or refuses `Network` as one capability, and the carriage states whether a
format is required. Every configured kind is a transport with a constructor
of the same name, so `source`, `tap`, `target` and `sink` take a configured
id, or a transport and a format, and reach the same systems either way.
`http`, `websocket` and `tcp` become configured kinds, and `io.net` gains
three transports over HTTP: `rest`, the items of a paginated collection,
`graphql`, the nodes of one query, and `webhook`, the bodies of the requests
the host receives.

## Motivation

- [HQL-0021](../HQL-0021/README.md) calls network endpoints "endpoints that
  are transports" and declares no type for it. Whether an endpoint is on the
  network is decided per constructor, by a check in `src/extensions/net.rs`,
  and per kind, by refusals in `src/io.rs`; no type states it.
- The systems a program reaches divide in two with no overlap. `file`, `http`,
  `websocket` and `tcp` exist only as constructors; `sqlite`, `postgres`
  and `kafka` exist only as kinds of `[endpoint.<id>]`.
- The network page of the book states that a request needing credentials is a
  configured endpoint. No `http` kind exists, so the only way to authenticate
  is a header written in the program.
- Whether a format is required is decided case by case: a file takes one, a
  WebSocket takes one per message, `sink("id")` over a table refuses one.

## Non-goals

- Renaming `Endpoint`, `Readable`, `Replayable` or `Writable`, or the `kind`
  key of a configured endpoint. The one rename is `socket` to `tcp`.
- New operations. `source`, `tap`, `target` and `sink` stay the four.
- Which module declares each transport: that is
  [HQL-0048](../HQL-0048/README.md).
- TLS and SASL to brokers and databases, consumer groups, and message keys,
  as [HQL-0046](../HQL-0046/README.md) excludes them.

## Specification

### 1. Terms

Each term has one meaning.

- **endpoint**: a value stating where values are. Constructing one performs
  no I/O ([HQL-0021](../HQL-0021/README.md), "Endpoints").
- **transport**: an endpoint outside the evaluation. A `Channel` is the
  endpoint that is not one: it holds values inside the evaluation.
- **configured endpoint**: a transport, its format where it takes one, and
  its schema boundary, declared under an id in `hql.toml`
  ([HQL-0012](../HQL-0012/README.md), [HQL-0035](../HQL-0035/README.md)).
- **extension transport**: how an extension's code reaches the process, in
  [HQL-0024, packages and extensions](../HQL-0024/parts/09-packages-and-extensions.md).
  It is unrelated. In `io`, in the book and in diagnostics, "transport" means
  the I/O transport of this record.

### 2. The types

`Transport` and its supertypes are declared in `io`, beside `Endpoint`.

```hql
abstract type Transport : Endpoint

// locality: what a host policy grants
abstract type Direct  : Transport   // this machine: process streams, the file system
abstract type Network : Transport   // another process, over a connection

// carriage: what one read or write moves
abstract type ByteTransport    : Transport   // bytes; a format separates documents
abstract type MessageTransport : Transport   // framed messages; a format decodes each
abstract type RowTransport     : Transport   // rows of named columns; no format
```

- Every concrete transport MUST have exactly one locality and exactly one
  carriage among its supertypes. The capabilities `Readable`, `Replayable`
  and `Writable` are unchanged and independent of both.
- `Channel<T>` MUST NOT be a `Transport`.
- The existing types gain supertypes and lose none:

| Type | Locality | Carriage | Capabilities |
| --- | --- | --- | --- |
| `Stdin` | `Direct` | bytes | `Readable` |
| `Stdout`, `Stderr` | `Direct` | bytes | `Writable` |
| `File` | `Direct` | bytes | `Replayable`, `Writable` |
| `FileEnd` | `Direct` | bytes | `Readable`, `Writable` |
| `Http` | `Network` | bytes | `Replayable`, `Writable` |
| `Tcp` | `Network` | bytes | `Readable`, `Writable` |
| `WebSocket` | `Network` | messages | `Readable`, `Writable` |
| `Sqlite` | `Direct` | rows | `Replayable`, `Writable` |
| `Postgres` | `Network` | rows | `Replayable`, `Writable` |
| `Spark` | `Network` | rows | `Replayable`, `Writable` |
| `Rest` | `Network` | messages | `Replayable` |
| `Graphql` | `Network` | messages | `Replayable` |
| `Webhook` | `Network` | messages | `Readable` |
| `Kafka` | `Network` | messages | `Readable`, `Writable` |
| `Nats` | `Network` | messages | `Readable`, `Writable` |

`Sqlite`, `Postgres`, `Spark`, `Kafka` and `Nats` are new types, one per
kind of `src/endpoints.rs`; the section "Kinds and constructors" declares
them.

### 3. Carriage and formats

The carriage states whether an operation takes a format.

- Over a `ByteTransport`, the format is required, except where
  [HQL-0021](../HQL-0021/README.md) gives a default, as it does for a
  `File` by its extension.
- Over a `MessageTransport`, each message is one document and the format
  decodes or encodes it. The format MAY be omitted and is then `Json`, which
  is what a topic of [HQL-0046](../HQL-0046/README.md) carries.
- Over a `RowTransport`, a format MUST be a check error naming the transport:
  a row has columns, and no bytes to decode.
- A configured id takes no format in the program, as today: the
  configuration states it.

### 4. The four operations

No header changes. `source` and `tap` take a `Readable` or an id, `target`
and `sink` a `Writable` or an id.

```hql
source("orders")                                   // a configured endpoint
source(file("birds.json"))                         // a byte transport, format by extension
source(http("https://api.example.com/birds"), Json)
source(postgres(dsn_env = "DEMO_DSN", table = "orders"))   // a row transport
tap(kafka(brokers = "localhost:9092", topic = "events"))   // a message transport
```

- `source` over a `Replayable` transport gives a `Seq`, and `tap` over a
  `Readable` one a `Stream`, by the rules of
  [HQL-0021](../HQL-0021/README.md).
- A transport constructed in a program has no schema boundary. A row is read
  as `Data`, or as the expected type by typed decoding; `fields`, `mode` and
  `strictness` are stated only in configuration.

### 5. Kinds and constructors

A kind and a constructor are two spellings of one transport.

- Every kind of `[endpoint.<id>]` other than `vault` MUST have a transport
  type and a constructor of the kind's name, whose parameters are the kind's
  own configuration keys, with the same names, defaults and checks.
- Every transport constructor other than the host-bound `stdin`, `stdout`
  and `stderr` MUST have a kind of its name. `http`, `websocket` and `tcp`
  therefore become kinds.
- The file kinds `csv`, `json`, `json_lines` and `yaml` stay as they are:
  each is a `File` with the format its name states.
- A kind whose transport is a `ByteTransport` or a `MessageTransport`, and
  whose name states no format, MUST take a `format` key.
- A constructor MUST NOT take a secret. A parameter that names one names the
  environment variable holding it, as `dsn_env` does, so a secret is never in
  a program or in `hql.toml`.
- A header is ordinary text in a program. A configured `http` or `websocket`
  kind MAY name `token_env`, as the `spark` kind does, sent as
  `Authorization: Bearer <value>`; a
  `headers` table whose key is a secret key of `src/endpoints.rs` refuses the
  vault, naming `token_env`.

```hql
fn sqlite(path: String, table: String, order: Seq<String> = []) : Sqlite
fn postgres(dsn_env: String, table: String, order: Seq<String> = []) : Postgres
fn kafka(topic: String, brokers: String = "", brokers_env: String = "",
         from: String = "earliest", idle_ms: Int = 0) : Kafka
```

`spark` and `nats` have constructors by the same rule, over the keys their
kinds take.

```toml
[endpoint.birds]
kind = "http"
format = "json"
url = "https://api.example.com/birds"
token_env = "BIRDS_TOKEN"

[endpoint.alerts]
kind = "websocket"
url = "wss://events.example.com"

[endpoint.feed]
kind = "tcp"
format = "json_lines"
host = "localhost"
port = 7000
```

- The constructor `socket(host, port)` of `io.net` is renamed
  `tcp(host, port)` and its type `Socket` is renamed `Tcp`: the name states
  the protocol, as `http` and `websocket` do. `socket` and `Socket` are
  removed, with no alias.

### 6. What a host grants

A host policy is stated over locality.

- A policy MUST grant or refuse `Network` as a whole. `hql render` refuses
  it: every use of a `Network` transport, constructed or configured, is a
  check error, "a document reads and writes nothing on the network".
- The check MUST read the locality from the transport's type or the kind's
  transport type, and from nothing else. The per-constructor check of
  `src/extensions/net.rs` and the per-kind refusals of `src/io.rs` are
  replaced by it.
- The rules for `Direct` transports are those of
  [HQL-0021](../HQL-0021/README.md), "Effects and what a host grants",
  unchanged.
- A switch that refuses `Network` under `hql eval` and `hql run` is design
  direction; its spelling belongs to the execution context
  ([HQL-0025](../HQL-0025/README.md)).

### 7. Network transports are bounded

A `Network` read or write ends.

- A connection attempt MUST fail after 10 seconds, and an `Http` response
  that sends nothing for 30 seconds MUST fail; each is an evaluation failure
  naming the URL or the address. A `tap` waits for the next message without
  limit, as a `Stream` does.
- An `Http` body is fetched once per evaluation for each distinct pair of
  URL and headers.
- A `Tcp` write MUST separate documents as the format does, a `---` line
  for `Yaml`, so what one program writes another reads.
- `max_bytes` bounds an `Http` body as it bounds a file.

### 8. REST, GraphQL and webhooks

Three transports of `io.net` read documents over HTTP. Each is a
`MessageTransport`: an item, a node or a request body is one document,
decoded as `Json` unless a format is named. Each is a constructor and a kind
of the same name and keys.

```hql
import net

type Rest    : {Network, MessageTransport, Replayable}
type Graphql : {Network, MessageTransport, Replayable}
type Webhook : {Network, MessageTransport, Readable}

fn rest(url: String, items: String = "", next: String = "",
        headers: Data = {:}) : Rest
fn graphql(url: String, query: String, variables: Data = {:},
           items: String = "", headers: Data = {:}) : Graphql
fn webhook(port: Int, path: String = "/", host: String = "127.0.0.1",
           idle_ms: Int = 0) : Webhook
```

```hql
source(rest("https://api.example.com/birds", items = "results", next = "link"))
source(graphql("https://api.example.com/graphql",
               query = "{ birds { name } }", items = "birds"))
tap(webhook(8080, path = "/sightings"))
```

**`rest`.**

- A read MUST send `GET` to `url` and decode the response body as one JSON
  document. The items are the elements of the array at the field path
  `items`, `"data.results"`, or the body itself when `items` is empty and
  the body is an array. A body with no array there is an evaluation failure
  naming the URL and the path.
- `next` states where the next page is: empty for one page; `"link"` for
  the `rel="next"` target of the `Link` response header; any other value a
  field path in the body holding the next page's URL. Pages are requested in
  turn until none is named, and `source` gives every item in page order.
- The pages are fetched once per evaluation, so a `Rest` is `Replayable`.
  `max_elements` and `max_bytes` bound the whole read; a read never ends
  early in silence.
- A `Rest` is not `Writable`. A request by another method is a write to an
  `http` endpoint.

**`graphql`.**

- A read MUST send one `POST` to `url` whose body is the JSON object of
  `query` and `variables`, with `Content-Type: application/json`.
- A response whose `errors` is a non-empty array is an evaluation failure
  naming the URL and the first error's `message`.
- The items are the elements of the array at the field path `items` below
  the response's `data`. A path that names no array is an evaluation
  failure.
- One read is one request. Following a cursor across requests is design
  direction.

**`webhook`.**

- A `tap` MUST listen on `host` and `port` from the first traversal until
  the evaluation ends, and give the body of each `POST` to `path` as one
  document, in the order the requests are accepted. It is not `Replayable`:
  `source` over it is a check error naming `tap`.
- A request is answered `202` once its body has been read and decoded, `400`
  where the body does not decode, `404` for another path, `405` for another
  method and `413` for a body above `max_bytes`. A body that does not decode
  is not an evaluation failure: the sender is answered and the stream
  continues.
- `host` is the loopback address unless written, so a webhook is reachable
  from another machine only where a program or a configuration says so.
- With `idle_ms` above 0 the stream ends once no request has arrived for
  that long; at 0 it does not end.
- A configured `webhook` MAY name `token_env`. A request whose
  `Authorization` header is not `Bearer` and that value is answered `401`
  and gives no document.

```toml
[endpoint.birds]
kind = "rest"
url = "https://api.example.com/birds"
items = "results"
next = "link"
token_env = "BIRDS_TOKEN"

[endpoint.sightings]
kind = "webhook"
port = 8080
path = "/sightings"
token_env = "HOOK_TOKEN"
```

## Backwards Compatibility

- `socket(host, port)` becomes `tcp(host, port)` and `Socket` becomes `Tcp`.
  A program that writes `socket` is a name error naming `tcp`. This is the
  one breaking change.
- No other program changes meaning. The existing types gain supertypes; no
  other type, operation, kind or key is renamed or removed.
- `source(http(u))` with no format is a check error today and stays one.
- The check error for a network endpoint in a document keeps its text.

## Security Considerations

- A constructor reaches a system without configuration, so the rule that no
  constructor takes a secret is what keeps credentials out of program text.
  A failure message never repeats a connection string or a header value.
- The network grant is one check over a type. A new transport is refused in
  documents by declaring `Network`, with no code of its own.
- Timeouts bound how long a server that does not answer holds an evaluation.
- A `webhook` is the one transport that accepts connections. It binds the
  loopback address by default, reads at most `max_bytes` of a body, runs
  nothing a request holds, and is refused in a document as every `Network`
  transport is.

## Deployment / Activation

Each step is committed with its tests before the next.

1. The rename of `socket` to `tcp`, in `lib/std/io/net/mod.hql`,
   `src/extensions/net.rs`, `src/io.rs`, the tests, the book and the
   reference pages, with a `CHANGELOG.md` entry under breaking changes.
2. Types: `Transport`, the two localities and the three carriages in
   `lib/std/io/mod.hql`, the supertypes on the existing types, and the host
   check over locality. No behaviour changes.
3. Network kinds: `http`, `websocket` and `tcp` in `src/endpoints.rs`,
   with `format`, `headers` and `token_env`.
4. The bounds of the section "Network transports are bounded".
5. `Sqlite`, `Postgres`, `Spark`, `Kafka` and `Nats` as types with
   constructors, in the
   modules [HQL-0048](../HQL-0048/README.md) names, each resolving to the
   reader and writer its kind already has.
6. The book: the network page gains a configured `http` endpoint, the
   endpoints page states locality and carriage, and the page on endpoints by
   id lists the three kinds.

7. `rest`, as a constructor and a kind, on the client `http` uses.
8. `graphql`, as a constructor and a kind.
9. `webhook`, as a constructor and a kind, on a listener of the Rust
   standard library; the book's network page states all three.

## Reference Implementation

- `lib/std/io/mod.hql`, `lib/std/io/net/mod.hql` for the types.
- `src/types/builtin.rs` for the type references, `src/io.rs` for
  `Endpoint`, its locality and carriage, and the one host check.
- `src/endpoints.rs` for the three kinds, `Kind` to `Endpoint` resolution
  and `token_env`.
- `src/net.rs` for timeouts, WebSocket handshake headers and the socket
  separator.
- `src/extensions/io.rs` for the format rule by carriage.
- `src/net.rs` for pagination, the GraphQL request and the webhook
  listener; `src/extensions/net.rs` for the three constructors;
  `lib/std/io/net/mod.hql` for their contracts.

## Test Plan

Unit and integration tests MUST include:

- each existing type's locality and carriage, by `hql check`;
- a format over a row transport refused, and a message transport read with
  no format as `Json`;
- a configured `http` id with `token_env` sending `Authorization`, and an
  unset variable as a failure naming it;
- a configured `websocket` and `tcp` id read by `tap`;
- every `Network` transport, constructed and configured, refused under
  `hql render` with the one message;
- a server that never answers ending in a failure naming the URL;
- `Yaml` documents written to a `tcp` endpoint and read back;
- `rest` over a loopback server: one page, pages by `Link`, pages by a body
  field, a body with no array at `items`, and a bound exceeded;
- `graphql` over a loopback server: the request body sent, the nodes read,
  and a response with `errors` as a failure naming its message;
- `webhook` on a loopback port: two posted bodies read in order, each status
  above, `token_env` refusing a request without the token, and `idle_ms`
  ending the stream.

```bash
cargo test --test net --test topics_and_tables --test tutorial
cargo clippy --all-targets -- -D warnings
```

## Changelog

- 2026-11-12: drafted
- 2026-11-13: implemented in part: `Transport`, `Direct`, `Network` and the
  carriages are declared in `io`, and `Sqlite`, `Postgres`, `Spark`, `Nats`
  and `NatsJetStream` are transport types with a constructor of their name.
  Not built: a locality and carriage for `Stdin`, `Stdout`, `Stderr`, `File`,
  `FileEnd`, `Http`, `WebSocket` and `Socket`; one `hql render` check over
  `Network`; `kafka`, `http`, `websocket` and `tcp` as
  transports; network timeouts ([issue 0069](../../issues/0069-transports-core.md))
- 2026-11-13: the rename built, in 0.6.0: `socket(host, port)` is
  `tcp(host, port)` and `Socket` is `Tcp`; each old name is refused with
  help naming the new one (issue 0072)
