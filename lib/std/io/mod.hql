// Input and output: endpoints, the data sources read from them, and the data
// targets written to them.
//
// See doc/proposals/HQL-0021/README.md and doc/proposals/HQL-0012/README.md.
//
// An endpoint says where bytes are and performs no I/O. A format type
// (module `format`) says how the bytes are documents. A source or target joins the
// two, and Seq and Stream say whether the documents end.
import core

// Where bytes are. Constructing one opens, stats and resolves nothing.
abstract type Endpoint

// An endpoint bytes can be read from.
abstract type Readable : Endpoint

// A readable endpoint that gives the same bytes on every read in one
// evaluation, so what it holds can be a Seq.
abstract type Replayable : Readable

// An endpoint bytes can be written to.
abstract type Writable : Endpoint

// The program's standard input: read once, as it arrives.
type Stdin : Readable

// The program's standard output: written while the program runs, before the
// host presents its result.
type Stdout : Writable

// The program's standard error: where diagnostics go, apart from the result.
type Stderr : Writable

// The end of a file, after what it holds: where an append goes, and where a
// reader waits for what is written next, as `tail -f` does. Live, so it is
// not Replayable, and it holds nothing to read as a Seq.
type FileEnd : {Readable, Writable} {
    path : String
}

// A local file, the whole of it: read, every document it holds; written, it
// is replaced. A relative path resolves against the directory of the source
// unit that names it: the document, the program file, or the working
// directory for an expression.
type File : {Replayable, Writable} {
    path : String
    end  : FileEnd   // file(p).end appends, and tap(file(p).end, F) follows
}

// An endpoint of values rather than bytes, held in memory for one execution
// scope: a program run, or one cell. It keeps T, so it takes no format.
// Values are read in the order they were written. Construct one with
// Channel<T>(name); one name is one channel within a scope.
type Channel<T> : {Readable, Writable} {
    name : String
}

// The element type is written at the call, `Channel<LogEvent>("logs")`:
// nothing written to the channel yet can say it.
fn Channel<T>(name: String) : Channel<T>

// Bound by the host, as `vault` is:
// stdin  : Stdin
// stdout : Stdout
// stderr : Stderr

// What a program reads: a configured source by id, or an endpoint read in a
// format. Its contract fixes what its contents are.
abstract type DataSource

// Every document, in order, and it ends: read from a Replayable endpoint.
abstract type FiniteSource<T> : {DataSource, Seq<T>}

// Each document as it arrives, with no replay: read from a live endpoint.
abstract type LiveSource<T> : {DataSource, Stream<T>}

// Finite contents addressable by key: get(s, key) gives Option<T>.
abstract type KeyedSource<K, T> : FiniteSource<T>

// A message's place in its log, in the log's order.
type Position : Orderable

// One element of a stream, with where and when it sits in it.
type Message<T> {
    value    : T
    position : Position
    key?     : Data
    time?    : Date
    headers  : Data
}

// A log: contents in order, replayable within its retention.
abstract type StreamSource<T> : {DataSource, Stream<Message<T>>}

// An endpoint that contains sources: a database of tables, a cluster of
// topics.
abstract type Catalog<S> : DataSource

// A vault: an endpoint that holds its cards, as a catalog holds tables,
// with its members as fields rather than names. It is not a Seq<Card>
// itself; its cards are. What the documents sit on, a filesystem, a git
// revision, an HTTP server, is the concrete type's. See HQL-0020.
abstract type VaultSource : DataSource

// A configured source, by its id: `source("field_notes")`. The run
// context's configuration, `[sources.<id>]` in hql.toml, states its kind,
// and the kind its type, read at check time and offline: a `vault` source
// is a Vault, and a `csv`, `json`, `json_lines`, `yaml` or `sqlite` source
// a Seq<Data>, or a Seq of the type its `element` names. The id is a
// literal, snake_case, and never a connection string; an id takes no
// format, since its kind fixes one.

// What a program writes to: an endpoint written in a format, and later a
// configured target by id, such as a table.
abstract type DataTarget

// The operations the `io` extension implements. Constructing an endpoint is
// pure; reading and writing one is effectful: it runs once, where it is
// written. `source` is a Seq read from a Replayable endpoint and a Stream
// read from a live one, and T is the type the format decodes to, or the
// element type the read is expected to give: `s : Seq<Sighting> =
// source(file("birds.json"))` decodes each document as a Sighting.
fn file(path: String) : File
effectful fn source<F : Format, T>(endpoint: Readable, format: Type<F>) : Iterable<T>
effectful fn source<F : Format, T>(id: String, format: Type<F>) : Iterable<T>
effectful fn tap<F : Format, T>(endpoint: Readable, format: Type<F>) : Stream<T>
effectful fn tap<F : Format, T>(id: String, format: Type<F>) : Stream<T>
effectful fn target<T, F : Format>(value: T, endpoint: Writable, format: Type<F>) : Unit
effectful fn target<T, F : Format>(value: T, id: String, format: Type<F>) : Unit
effectful fn sink<T, F : Format>(values: Iterable<T>, endpoint: Writable, format: Type<F>) : Unit

// Reading, provided by the `io` extension. The format may be omitted only
// when the endpoint is `file` of a literal path whose extension implies one.
// source<T>(e: Replayable, format: Type<F>) : FiniteSource<T>   every document
// source<T>(e: Readable, format: Type<F>)   : LiveSource<T>     each as it arrives
// source<S>(id: String) : S                                    configured
// source<T>(c: Channel<T>) : Seq<T>                            what it holds now
// tap<T>(e: Readable, format: Type<F>) : Stream<T>             one document held at a time
// tap<T>(id: String) : Stream<T>
// tap<T>(c: Channel<T>) : Stream<T>                            each value as it is written
//
// Writing, provided by the `io` extension. Both return Unit and are Effectful.
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
