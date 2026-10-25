// See doc/proposals/HQL-0021/README.md and doc/proposals/HQL-0012/README.md.

/// Input and output: endpoints, the data sources read from them, and the
/// data targets written to them. An endpoint names where bytes are and
/// performs no I/O; a format type (module `format`) states how the bytes are
/// documents; a source or target joins the two, and `Seq` and `Stream` state
/// whether the documents end.
module io

import core

/// The abstract type of every endpoint: where bytes are. Constructing one
/// opens, stats and resolves nothing.
abstract type Endpoint

/// An endpoint bytes can be read from, by `source` and `tap`.
abstract type Readable : Endpoint

/// A `Readable` endpoint that gives the same bytes on every read in one
/// evaluation, so `source` reads what it holds as a `Seq`.
abstract type Replayable : Readable

/// An endpoint bytes can be written to, by `target` and `sink`.
abstract type Writable : Endpoint

/// The program's standard input, bound as `stdin`: read once, as it
/// arrives.
type Stdin : Readable

/// The program's standard output, bound as `stdout`: written while the
/// program runs, before the host presents its result.
type Stdout : Writable

/// The program's standard error, bound as `stderr`: where diagnostics go,
/// apart from the result.
type Stderr : Writable

/// The end of a file, after what it holds, as `file(path).end`: a write
/// there appends, and `tap` follows what is written next, as `tail -f`
/// does. It is live, not `Replayable`, so `source` of it is a type error.
type FileEnd : {Readable, Writable} {
    path : String
}

/// A local file, the whole of it: read, every document it holds; written,
/// it is replaced. A relative `path` resolves against the directory of the
/// source unit that names it: the document, the program file, or the
/// working directory for an expression.
type File : {Replayable, Writable} {
    path : String
    end  : FileEnd   // file(p).end appends, and tap(file(p).end, F) follows
}

/// An endpoint of values rather than bytes, held in memory for one
/// execution scope: a program run, or one cell. It holds `T`, so it takes
/// no format, and its values are read in the order they were written. One
/// `name` is one channel within a scope.
type Channel<T> : {Readable, Writable} {
    name : String
}

/// The channel named `name` in the current scope, holding `T`. The element
/// type is written at the call, `Channel<LogEvent>("logs")`, since nothing
/// written to the channel yet determines it.
fn Channel<T>(name: String) : Channel<T>

// Bound by the host, as `vault` is:
// stdin  : Stdin
// stdout : Stdout
// stderr : Stderr

/// What a program reads: a configured source by id, or an endpoint read in
/// a format. Its contract fixes what its contents are.
abstract type DataSource

/// A source of every document, in order, that ends: read from a
/// `Replayable` endpoint.
abstract type FiniteSource<T> : {DataSource, Seq<T>}

/// A source of each document as it arrives, with no replay: read from a
/// live endpoint.
abstract type LiveSource<T> : {DataSource, Stream<T>}

/// A finite source whose contents are addressable by a key of type `K`.
abstract type KeyedSource<K, T> : FiniteSource<T>

/// A message's place in its log, ordered as the log is.
type Position : Orderable

/// One element of a stream with where and when it sits in it: its `value`,
/// its `position`, an optional `key` and `time`, and its `headers`.
type Message<T> {
    value    : T
    position : Position
    key?     : Data
    time?    : Date
    headers  : Data
}

/// A log: `Message<T>`s in order, replayable within its retention.
abstract type StreamSource<T> : {DataSource, Stream<Message<T>>}

/// An endpoint that contains sources of type `S`: a database of tables, a
/// cluster of topics.
abstract type Catalog<S> : DataSource

// See HQL-0020.
/// A vault as a source: an endpoint that holds its cards, as a catalog
/// holds tables, with its members as fields rather than names. It is not a
/// `Seq<Card>` itself; its cards are. What the documents sit on, a
/// filesystem, a git revision, an HTTP server, is the concrete type's.
abstract type VaultSource : DataSource

// A configured source, by its id: `source("field_notes")`. The run
// context's configuration, `[sources.<id>]` in hql.toml, states its kind,
// and the kind its type, read at check time and offline: a `vault` source
// is a Vault, and a `csv`, `json`, `json_lines`, `yaml` or `sqlite` source
// a Seq<Data>, or a Seq of the type its `element` names. The id is a
// literal, snake_case, and never a connection string; an id takes no
// format, since its kind fixes one.

/// What a program writes to: an endpoint written in a format, or a
/// configured target by its id.
abstract type DataTarget

// The operations the `std` extension implements.

/// The `File` at `path`. Constructing it is pure and opens nothing:
/// `file("birds.json").path` is `"birds.json"`. A URL is a type error when it
/// is a literal and an evaluation failure when it is computed; a network
/// resource is read with `http`.
fn file(path: String) : File

/// Every document read from `endpoint` in `format`, decoded to `T`: a `Seq`
/// from a `Replayable` endpoint, a `Stream` from a live one. `T` is the
/// type the format decodes to, or the element type the read is expected to
/// give: `s : Seq<Sighting> = source(file("birds.json"))` decodes each
/// document as a `Sighting`. The format may be omitted when the endpoint is
/// `file` of a literal path whose extension implies one. `source(id)` reads
/// the source configured under the literal `id` in hql.toml, as the type
/// its kind states, and takes no format; `source(c)` of a `Channel<T>` is a
/// `Seq<T>` of what it holds now. Effectful: it runs once, where it is
/// written.
effectful fn source<F : Format, T>(endpoint: Readable, format: Type<F>) : Iterable<T>
effectful fn source<F : Format, T>(id: String, format: Type<F>) : Iterable<T>

/// The documents read from `endpoint` in `format`, one held at a time, as a
/// `Stream<T>`. A format that is one whole document, such as `Text`, is
/// a type error. `tap(id)` gives the elements of the finite source configured
/// under `id` as a `Stream`; `tap(c)` of a `Channel<T>` gives each value as
/// it is written; `tap(file(path).end, F)` follows what is appended.
/// Effectful: it runs once, where it is written.
effectful fn tap<F : Format, T>(endpoint: Readable, format: Type<F>) : Stream<T>
effectful fn tap<F : Format, T>(id: String, format: Type<F>) : Stream<T>

/// Writes `value` as one document to `endpoint` in `format`, or to the
/// target configured under `id`, and gives `Unit`. A `Seq` is one
/// document, in `Json` an array; a `Stream` that is not a `Seq` is a type error:
/// write it with `sink`, or bound it with `take(n)` first. Written to a
/// `Channel<T>`, `value` is appended and no format is taken. Effectful: it
/// runs once, where it is written.
effectful fn target<T, F : Format>(value: T, endpoint: Writable, format: Type<F>) : Unit
effectful fn target<T, F : Format>(value: T, id: String, format: Type<F>) : Unit

/// Writes each of `values` to `endpoint` in `format`, one document per
/// value, and gives `Unit`: `[{a: 1}, {a: 2}] | sink(stdout, Json)` writes
/// `{"a":1}` and `{"a":2}` on two lines. Written to a `Channel<T>`, each
/// value is appended and no format is taken. Effectful: it runs once,
/// where it is written.
effectful fn sink<T, F : Format>(values: Iterable<T>, endpoint: Writable, format: Type<F>) : Unit

// Reading, provided by the `std` extension. The format may be omitted only
// when the endpoint is `file` of a literal path whose extension implies one.
// source<T>(e: Replayable, format: Type<F>) : FiniteSource<T>   every document
// source<T>(e: Readable, format: Type<F>)   : LiveSource<T>     each as it arrives
// source<S>(id: String) : S                                    configured
// source<T>(c: Channel<T>) : Seq<T>                            what it holds now
// tap<T>(e: Readable, format: Type<F>) : Stream<T>             one document held at a time
// tap<T>(id: String) : Stream<T>
// tap<T>(c: Channel<T>) : Stream<T>                            each value as it is written
//
// Writing, provided by the `std` extension. Both return Unit and are Effectful.
// target<T>(x: T, e: Writable, format: Type<F>) : Unit           x as one document
// sink<T>(xs: Stream<T>, e: Writable, format: Type<F>) : Unit    a document per value
// target<T>(x: T, c: Channel<T>) : Unit                          appends x
// sink<T>(xs: Stream<T>, c: Channel<T>) : Unit                   appends each value
//
// Without the values, sink(e, F) is a step: Stream<T> -> Unit. A Logger
// (module `io.logging`) takes one as its destination.
//
// What each accepts, by the shape of the values:
//
//                     Seq<T>                      Stream<T>, not a Seq
//   source            from a Replayable endpoint  from a live endpoint
//   tap               read one at a time          as it is
//   target            one document: a Json array  refused: collect, or take(n)
//   sink              one document per element    one document per value
//   result, --to      one whole text              one piece per value
