<!-- hql reference: std/format -->
# `std/format`

```hql
module format
```

The formats, how bytes are read as values and how values are written as
text, and the codec contracts over them.

A format is a type narrowing `Format`, the category `core` declares. It
names an external representation and, where it applies, its framing:
`Lines` is text framed one record per line, `JsonLines` one JSON document
per line. A format is not the text it encodes, so none narrows `String`,
and it is not `Format<T>`, since one format encodes many value
structures. Whether what is read ends is a property of the source, not of
the format: `source(file(p), Lines)` is a `Seq<String>` and
`source(stdin, Lines)` a `Stream<String>`.

A `String` is always UTF-8. Every format that holds text reads and writes
UTF-8, and decoding bytes that are not UTF-8 is a decoding error.

- [`Text`](types/Text.md) — The plain-text format: read, the whole input is one `String`; written, a value is its display text, and a `String` its characters as they are.
- [`Lines`](types/Lines.md) — The format of text framed one record per line: read, each line is one `String`; written, each element of a collection is one line: `[1, 2] | lines` is `"1\n2"`.
- [`Hql`](types/Hql.md) — The format of HQL's own value notation: `{a: 1, b: [true]} | hql` is `"{a: 1, b: [true]}"`.
- [`Markdown`](types/Markdown.md) — The Markdown format: read, the whole input is one `String`; written, a collection is a list, one item per element: `[1, 2] | markdown` is `"- 1\n- 2"`.
- [`Hmd`](types/Hmd.md) — The HyperMarkDown format, a narrowing of `Markdown`: every HyperMarkDown text is Markdown, and a value is written in it as in `Markdown`.
- [`Json`](types/Json.md) — The JSON format: read, each document is one `Data` value; written, a value is one indented JSON document.
- [`JsonLines`](types/JsonLines.md) — The JSON Lines format, one JSON document per line: read, each line is one `Data` value; written, each element of a collection is one line of compact JSON: `[{a: 1}, {a: 2}] | jsonlines` is `"{\"a\":1}\n{\"a\":2}"`.
- [`Yaml`](types/Yaml.md) — The YAML format: read, each document of the input, `---` separating them, is one `Data` value; written, a value is one YAML document.
- [`Csv`](types/Csv.md) — The comma-separated values format, with a header row: read, each row is one `Data` value whose fields, named by the header, hold `String`s; written, a collection of records or a `Table` is the header and one row per element.
- [`D2`](types/D2.md) — The D2 diagram format, which writes a `Graph`, or a `Data` value holding one, and nothing else.
- [`Mermaid`](types/Mermaid.md) — The Mermaid diagram format, which writes a `Graph`, or a `Data` value holding one, and nothing else.
- [`DiagnosticText`](types/DiagnosticText.md) — The text form of a `Diagnostic` (HQL-0045), which writes a diagnostic, or a sequence of them, and nothing else: each one's heading, `phase severity[kind]: message`, its origin and the fields that apply, then a count per severity when there is more than one.
- [`Table`](types/Table.md) — Rows of type `R` under ordered columns: the shape every format's encoder writes as a table.
- [`Decoder`](types/Decoder.md) — The contract that a format's records are read as values of `T`: `source(e, F)` gives the `T` of `F`'s decoder.
- [`Encoder`](types/Encoder.md) — The contract that values of `T` are written as records of a format: `format(x, F)` takes the types `F`'s encoders cover.
- [`format`](format.md) — The formats, how bytes are read as values and how values are written as text, and the codec contracts over them.
- [`text`](text.md) — `x` encoded as `Text`: `x | text` is `format(x, Text)`.
- [`hql`](hql.md) — `x` encoded as `Hql`: `x | hql` is `format(x, Hql)`.
- [`markdown`](markdown.md) — `x` encoded as `Markdown`: `x | markdown` is `format(x, Markdown)`.
- [`hmd`](hmd.md) — `x` encoded as `Hmd`: `x | hmd` is `format(x, Hmd)`.
- [`json`](json.md) — `x` encoded as `Json`: `x | json` is `format(x, Json)`.
- [`yaml`](yaml.md) — `x` encoded as `Yaml`: `x | yaml` is `format(x, Yaml)`.
- [`csv`](csv.md) — `x` encoded as `Csv`: `x | csv` is `format(x, Csv)`.
- [`lines`](lines.md) — `x` encoded as `Lines`: `x | lines` is `format(x, Lines)`.
- [`jsonlines`](jsonlines.md) — `x` encoded as `JsonLines`: `x | jsonlines` is `format(x, JsonLines)`.
- [`d2`](d2.md) — `x` encoded as `D2`: `x | d2` is `format(x, D2)`.
- [`mermaid`](mermaid.md) — `x` encoded as `Mermaid`: `x | mermaid` is `format(x, Mermaid)`.

## Implementations

```hql
impl Decoder<String> for Text
impl Decoder<String> for Lines
impl Decoder<String> for Hql
impl Decoder<String> for Markdown
impl Decoder<String> for Hmd
```

`Text`, `Lines`, `Hql`, `Markdown` and `Hmd` are read as `String`s.

```hql
impl Decoder<Data> for Json
impl Decoder<Data> for JsonLines
impl Decoder<Data> for Yaml
impl Decoder<Data> for Csv
```

`Json`, `JsonLines`, `Yaml` and `Csv` are read as `Data`.

```hql
impl<T> Encoder<T> for Text
impl<T> Encoder<T> for Lines
impl<T> Encoder<T> for Hql
impl<T> Encoder<T> for Markdown
impl<T> Encoder<T> for Hmd
impl<T> Encoder<T> for Json
impl<T> Encoder<T> for JsonLines
impl<T> Encoder<T> for Yaml
impl<T> Encoder<T> for Csv
```

`Text`, `Lines`, `Hql`, `Markdown`, `Hmd`, `Json`, `JsonLines`, `Yaml`
and `Csv` write a value of any type.

```hql
impl Encoder<Graph> for D2
impl Encoder<Data> for D2
impl Encoder<Graph> for Mermaid
impl Encoder<Data> for Mermaid
```

`D2` and `Mermaid` describe graphs: they write a `Graph`, or a `Data`
value holding one.

```hql
impl Encoder<Diagnostic> for DiagnosticText
```

`DiagnosticText` writes a `Diagnostic`, and a sequence of them one after
another.
<!-- hql reference: end -->
