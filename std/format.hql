// Formats: how bytes are documents, and how values become bytes.
//
// A format is a value of Format, a category of its own. It describes an
// external representation and, where it applies, its framing: Lines is text
// framed one record per line, JsonLines one JSON document per line. A format
// is not the text it encodes, so none narrows String, and it is not Format<T>,
// since one format encodes many value structures. A format does not say
// whether what is read ends: the source decides that, so source(file(p),
// Lines) is a Seq<String> and source(stdin, Lines) a Stream<String>.
//
// See doc/proposals/HQL-0024/parts/07-formats-and-codecs.md and
// doc/proposals/HQL-0024/encode-decode-format.md.

abstract type Format

type Text      : Format
type Lines     : Format
// HQL's own literals: the text a program writes to produce the value.
type Hql       : Format
type Markdown  : Format
// Every HyperMarkDown text is Markdown.
type Hmd       : Markdown
type Json      : Format
type JsonLines : Format
type Yaml      : Format
type Csv       : Format
type D2        : Format
type Mermaid   : Format

// Rows under ordered columns: the shape every format's encoder writes as a
// table. A row is a record or a Data map, and `columns` names its fields in
// the order they are written; without it, the columns are the first row's
// fields in the order the row holds them. Over a declared record, a column
// naming no field is a type error; over Data, a missing field is an empty
// cell.
type Table

fn Table<T>(rows: Collection<T>, columns: List<String> = []) : Table

// The codec contracts, over the format: a Decoder<T> reads records of the
// format as values of T, an Encoder<T> writes values of T as records of it.
// HQL declares each codec as a bodyless header and an extension implements
// it: parsing, encoding, framing and buffering stay native. Binding holds
// these headers and the runtime's decoder and encoder tables to each other,
// so `source(e, F)` reads the T of F's decoder and `format(x, F)` takes
// what F's encoders cover. A format writes one type in several ways, so a
// format has an Encoder at every type it writes, and a renderer chosen for a
// type in a format is one of them.
trait Decoder<T> {}
trait Encoder<T> {}

impl Decoder<String> for Text
impl Decoder<String> for Lines
impl Decoder<String> for Hql
impl Decoder<String> for Markdown
impl Decoder<String> for Hmd
impl Decoder<Data> for Json
impl Decoder<Data> for JsonLines
impl Decoder<Data> for Yaml
impl Decoder<Data> for Csv

impl<T> Encoder<T> for Text
impl<T> Encoder<T> for Lines
impl<T> Encoder<T> for Hql
impl<T> Encoder<T> for Markdown
impl<T> Encoder<T> for Hmd
impl<T> Encoder<T> for Json
impl<T> Encoder<T> for JsonLines
impl<T> Encoder<T> for Yaml
impl<T> Encoder<T> for Csv
// D2 and Mermaid describe graphs: a Graph, or a document holding one.
impl Encoder<Graph> for D2
impl Encoder<Data> for D2
impl Encoder<Graph> for Mermaid
impl Encoder<Data> for Mermaid

// Encoding a value in a format, provided by the `format` module. The result
// is the encoded text, a String that keeps its format for presentation. A
// stream that may not end is encoded piece by piece, as it is consumed, so
// it gives a stream of texts; a collection, which ends, and any other value
// give one text. A `*` opens the arguments a call supplies by name only:
// `format(Markdown, limit = 3)`.
fn format<T, F : Format>(x: Collection<T>, as: Type<F>, *, limit: Int = 0, renderer: String = "") : String
fn format<T, F : Format>(x: Stream<T>, as: Type<F>, *, limit: Int = 0, renderer: String = "") : Stream<String>
fn format<T, F : Format>(x: T, as: Type<F>, *, limit: Int = 0, renderer: String = "") : String

// Each shorthand fixes the format and nothing else: `json` is `format(Json)`.
fn text<T>(x: Collection<T>) : String
fn text<T>(x: Stream<T>) : Stream<String>
fn text<T>(x: T) : String
fn hql<T>(x: Collection<T>) : String
fn hql<T>(x: Stream<T>) : Stream<String>
fn hql<T>(x: T) : String
fn markdown<T>(x: Collection<T>) : String
fn markdown<T>(x: Stream<T>) : Stream<String>
fn markdown<T>(x: T) : String
fn hmd<T>(x: Collection<T>) : String
fn hmd<T>(x: Stream<T>) : Stream<String>
fn hmd<T>(x: T) : String
fn json<T>(x: Collection<T>) : String
fn json<T>(x: Stream<T>) : Stream<String>
fn json<T>(x: T) : String
fn yaml<T>(x: Collection<T>) : String
fn yaml<T>(x: Stream<T>) : Stream<String>
fn yaml<T>(x: T) : String
fn csv<T>(x: Collection<T>) : String
fn csv<T>(x: Stream<T>) : Stream<String>
fn csv<T>(x: T) : String
fn lines<T>(x: Collection<T>) : String
fn lines<T>(x: Stream<T>) : Stream<String>
fn lines<T>(x: T) : String
fn jsonlines<T>(x: Collection<T>) : String
fn jsonlines<T>(x: Stream<T>) : Stream<String>
fn jsonlines<T>(x: T) : String
fn d2<T>(x: Collection<T>) : String
fn d2<T>(x: Stream<T>) : Stream<String>
fn d2<T>(x: T) : String
fn mermaid<T>(x: Collection<T>) : String
fn mermaid<T>(x: Stream<T>) : Stream<String>
fn mermaid<T>(x: T) : String
