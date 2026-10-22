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
// is the encoded text, a String that keeps its format for presentation:
// format<T, F : Format>(x: T, as: Type<F>, limit: Int = 0, renderer: String = "") : String
// Each shorthand fixes the format and nothing else:
// text     := format(Text)
// hql      := format(Hql)
// markdown := format(Markdown)
// hmd      := format(Hmd)
// json     := format(Json)
// yaml     := format(Yaml)
// csv      := format(Csv)
// d2       := format(D2)
// mermaid  := format(Mermaid)
