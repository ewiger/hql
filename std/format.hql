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

// The codec contracts. A Decoder<F, T> reads records of the format F as
// values of T; an Encoder<T, F> writes values of T as records of F. HQL
// declares the contract and an extension implements it: parsing, encoding,
// framing and buffering stay native. The decoders provided today:
//   Decoder<Lines, String>  Decoder<Text, String>
//   Decoder<Json, Data>     Decoder<JsonLines, Data>
//   Decoder<Yaml, Data>     Decoder<Csv, Data>
//   Decoder<Markdown, String>  Decoder<Hmd, String>  Decoder<Hql, String>
// Every format has an encoder for every value, apart from D2 and Mermaid,
// which encode a Graph alone.
abstract type Decoder<F, T>
abstract type Encoder<T, F>

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
