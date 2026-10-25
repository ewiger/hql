// Implemented by the `std` extension. Another external text encoding than
// UTF-8 is converted at the codec, never held in a String.
//
// See doc/proposals/HQL-0024/parts/07-formats-and-codecs.md and
// doc/proposals/HQL-0024/encode-decode-format.md.

/// The formats, how bytes are read as values and how values are written as
/// text, and the codec contracts over them.
///
/// A format is a type narrowing `Format`, the category `core` declares. It
/// names an external representation and, where it applies, its framing:
/// `Lines` is text framed one record per line, `JsonLines` one JSON document
/// per line. A format is not the text it encodes, so none narrows `String`,
/// and it is not `Format<T>`, since one format encodes many value
/// structures. Whether what is read ends is a property of the source, not of
/// the format: `source(file(p), Lines)` is a `Seq<String>` and
/// `source(stdin, Lines)` a `Stream<String>`.
///
/// A `String` is always UTF-8. Every format that holds text reads and writes
/// UTF-8, and decoding bytes that are not UTF-8 is a decoding error.
module format

import core
import graph

/// The plain-text format: read, the whole input is one `String`; written, a
/// value is its display text, and a `String` its characters as they are.
type Text      : Format

/// The format of text framed one record per line: read, each line is one
/// `String`; written, each element of a collection is one line:
/// `[1, 2] | lines` is `"1\n2"`.
type Lines     : Format

/// The format of HQL's own value notation:
/// `{a: 1, b: [true]} | hql` is `"{a: 1, b: [true]}"`. Read, the whole input
/// is one `String`.
type Hql       : Format

/// The Markdown format: read, the whole input is one `String`; written, a
/// collection is a list, one item per element:
/// `[1, 2] | markdown` is `"- 1\n- 2"`.
type Markdown  : Format

/// The HyperMarkDown format, a narrowing of `Markdown`: every HyperMarkDown
/// text is Markdown, and a value is written in it as in `Markdown`.
type Hmd       : Markdown

/// The JSON format: read, each document is one `Data` value; written, a
/// value is one indented JSON document.
type Json      : Format

/// The JSON Lines format, one JSON document per line: read, each line is one
/// `Data` value; written, each element of a collection is one line of
/// compact JSON: `[{a: 1}, {a: 2}] | jsonlines` is `"{\"a\":1}\n{\"a\":2}"`.
type JsonLines : Format

/// The YAML format: read, each document of the input, `---` separating
/// them, is one `Data` value; written, a value is one YAML document.
type Yaml      : Format

/// The comma-separated values format, with a header row: read, each row is
/// one `Data` value whose fields, named by the header, hold `String`s;
/// written, a collection of records or a `Table` is the header and one row
/// per element.
type Csv       : Format

/// The D2 diagram format, which writes a `Graph`, or a `Data` value holding
/// one, and nothing else.
type D2        : Format

/// The Mermaid diagram format, which writes a `Graph`, or a `Data` value
/// holding one, and nothing else.
type Mermaid   : Format

/// Rows of type `R` under ordered columns: the shape every format's encoder
/// writes as a table. `R` is a record type or `Data`; `Table` written
/// without it is `Table<Data>`.
type Table<R>

/// A `Table` of `rows`, its columns named by `columns` in the order they are
/// written; with `columns` empty, the columns are the first row's fields in
/// the order the row holds them. Over a declared record, a column naming no
/// field is a type error; over `Data`, a missing field is an empty cell.
/// `Table([{a: 1, b: 2}], ["b"]) | csv` is the header `b` and the row `2`.
fn Table<T>(rows: Collection<T>, columns: List<String> = []) : Table<T>

// HQL declares each codec as a bodyless header and an extension implements
// it: parsing, encoding, framing and buffering stay native. Binding holds
// these headers and the runtime's decoder and encoder tables to each other.

/// The contract that a format's records are read as values of `T`:
/// `source(e, F)` gives the `T` of `F`'s decoder.
trait Decoder<T> {}

/// The contract that values of `T` are written as records of a format:
/// `format(x, F)` takes the types `F`'s encoders cover. A format writes one
/// type in several ways, so a format has an `Encoder` at every type it
/// writes, and a renderer chosen for a type in a format is one of them.
trait Encoder<T> {}

/// `Text`, `Lines`, `Hql`, `Markdown` and `Hmd` are read as `String`s.
impl Decoder<String> for Text
impl Decoder<String> for Lines
impl Decoder<String> for Hql
impl Decoder<String> for Markdown
impl Decoder<String> for Hmd

/// `Json`, `JsonLines`, `Yaml` and `Csv` are read as `Data`.
impl Decoder<Data> for Json
impl Decoder<Data> for JsonLines
impl Decoder<Data> for Yaml
impl Decoder<Data> for Csv

/// `Text`, `Lines`, `Hql`, `Markdown`, `Hmd`, `Json`, `JsonLines`, `Yaml`
/// and `Csv` write a value of any type.
impl<T> Encoder<T> for Text
impl<T> Encoder<T> for Lines
impl<T> Encoder<T> for Hql
impl<T> Encoder<T> for Markdown
impl<T> Encoder<T> for Hmd
impl<T> Encoder<T> for Json
impl<T> Encoder<T> for JsonLines
impl<T> Encoder<T> for Yaml
impl<T> Encoder<T> for Csv

/// `D2` and `Mermaid` describe graphs: they write a `Graph`, or a `Data`
/// value holding one.
impl Encoder<Graph> for D2
impl Encoder<Data> for D2
impl Encoder<Graph> for Mermaid
impl Encoder<Data> for Mermaid

/// `x` encoded in the format `as`: the encoded text, a `String` that keeps
/// its format for presentation. A collection, which ends, and any other
/// value give one text; a `Stream`, which may not end, is encoded piece by
/// piece as it is consumed and gives a stream of texts. `limit`, when not
/// `0`, keeps that many top-level elements and notes how many were cut;
/// `renderer` names one of the format's renderers. The arguments after `*`
/// are supplied by name only: `[1, 2] | format(Json)` is `"[\n  1,\n  2\n]"`.
fn format<T, F : Format>(x: Collection<T>, as: Type<F>, *, limit: Int = 0, renderer: String = "") : String
fn format<T, F : Format>(x: Stream<T>, as: Type<F>, *, limit: Int = 0, renderer: String = "") : Stream<String>
fn format<T, F : Format>(x: T, as: Type<F>, *, limit: Int = 0, renderer: String = "") : String

/// `x` encoded as `Text`: `x | text` is `format(x, Text)`.
fn text<T>(x: Collection<T>) : String
fn text<T>(x: Stream<T>) : Stream<String>
fn text<T>(x: T) : String

/// `x` encoded as `Hql`: `x | hql` is `format(x, Hql)`.
fn hql<T>(x: Collection<T>) : String
fn hql<T>(x: Stream<T>) : Stream<String>
fn hql<T>(x: T) : String

/// `x` encoded as `Markdown`: `x | markdown` is `format(x, Markdown)`.
fn markdown<T>(x: Collection<T>) : String
fn markdown<T>(x: Stream<T>) : Stream<String>
fn markdown<T>(x: T) : String

/// `x` encoded as `Hmd`: `x | hmd` is `format(x, Hmd)`.
fn hmd<T>(x: Collection<T>) : String
fn hmd<T>(x: Stream<T>) : Stream<String>
fn hmd<T>(x: T) : String

/// `x` encoded as `Json`: `x | json` is `format(x, Json)`.
fn json<T>(x: Collection<T>) : String
fn json<T>(x: Stream<T>) : Stream<String>
fn json<T>(x: T) : String

/// `x` encoded as `Yaml`: `x | yaml` is `format(x, Yaml)`.
fn yaml<T>(x: Collection<T>) : String
fn yaml<T>(x: Stream<T>) : Stream<String>
fn yaml<T>(x: T) : String

/// `x` encoded as `Csv`: `x | csv` is `format(x, Csv)`.
fn csv<T>(x: Collection<T>) : String
fn csv<T>(x: Stream<T>) : Stream<String>
fn csv<T>(x: T) : String

/// `x` encoded as `Lines`: `x | lines` is `format(x, Lines)`.
fn lines<T>(x: Collection<T>) : String
fn lines<T>(x: Stream<T>) : Stream<String>
fn lines<T>(x: T) : String

/// `x` encoded as `JsonLines`: `x | jsonlines` is `format(x, JsonLines)`.
fn jsonlines<T>(x: Collection<T>) : String
fn jsonlines<T>(x: Stream<T>) : Stream<String>
fn jsonlines<T>(x: T) : String

/// `x` encoded as `D2`: `x | d2` is `format(x, D2)`.
fn d2<T>(x: Collection<T>) : String
fn d2<T>(x: Stream<T>) : Stream<String>
fn d2<T>(x: T) : String

/// `x` encoded as `Mermaid`: `x | mermaid` is `format(x, Mermaid)`.
fn mermaid<T>(x: Collection<T>) : String
fn mermaid<T>(x: Stream<T>) : Stream<String>
fn mermaid<T>(x: T) : String
