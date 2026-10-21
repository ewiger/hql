# Part 7: Formats and codecs

- **Source**: [encode, decode and format](../encode-decode-format.md), the
  record this part adopts, and [design draft 1](../draft/design-draft-1.md),
  decisions 60 to 65 and 49 to 52 of the Decision section
- **Depends on**: [the standard library](03-standard-library.md) for the
  `format.*` modules, [traits](05-traits.md) for the codec contracts
- **Needed by**: [packages](09-packages-and-extensions.md) (a package may
  provide codecs)

## Scope

What a **format** is, what it is not, and how values cross between HQL and an
external representation. This part is adopted as
[encode, decode and format](../encode-decode-format.md) states it; the rules
below restate that document against the implemented language.

## Rules

### `Format` is a category of its own

`Format` is a language-level abstract type. A format is a value of it, named
by a type: `Text`, `Lines`, `Json`, `JsonLines`, `Yaml`, `Csv`, `Markdown`,
`Hmd`, `Hql`, `D2`, `Mermaid`, each declared `: Format` in
`std/format.hql`. It is **not** `Format<T>`, and a format is not a subtype of
the values it encodes or decodes:

```hql
abstract type Format

type Lines : Format
type Json  : Format
```

`Lines : String` and `Lines : Seq<String>` are both refused as a model,
because each mixes **representation** with **data**. A format describes an
external representation and, where applicable, its framing rules.

### Decoders and encoders

```text
bytes -> Format + Decoder -> HQL values
HQL values + Encoder + Format -> bytes
```

The typed relationship is a pair of contracts:

```hql
abstract type Decoder<F, T>    // F : Format, decodes to T
abstract type Encoder<T, F>    // F : Format, encodes a T
```

`Decoder<Lines, String>` and `Encoder<String, Lines>` say that `Lines`
decodes logical line records into `String` values and encodes `String` values
back into line records. The format holds no implementation: HQL declares the
contract, an extension declares that it implements it, and the codec itself,
parsing, encoding, framing, buffering and the low-level I/O mechanics, lives
in Rust. The public interface is HQL; the native detail stays inside the
extension, as [modules](01-modules.md) has it for every external declaration.

### Framing belongs to the format

Reading `foo\nbar\nbaz\n` through `Lines` produces `["foo", "bar", "baz"]`,
not the lines with their breaks: the newline belongs to the representation.
Writing those three values through `Lines` produces the separators again.
`\n` against `\r\n`, UTF-8 decoding, buffering a partial record and the final
unterminated line are the codec's business; a program does not write
`split("\n")` to read a file as lines.

### A format does not determine cardinality

`source(file("input.txt"), Lines)` is a `Seq<String>` because a file is
finite; `source(stdin, Lines)` is a `Stream<String>` because standard input
streams. `Lines` itself is neither a `Seq<String>` nor a `Stream<String>`:

```text
Source    transport, bounded or unbounded, execution topology
Format    external representation, framing
Decoder   representation -> HQL values
Encoder   HQL values -> representation
```

The source and the sink decide whether the result is a `Seq<T>` or a
`Stream<T>`, as [HQL-0021](../../HQL-0021/README.md) already has it.

### JSON shows the same split

`source(file("data.json"), Json)` decodes one JSON document into `Data`.
`JsonLines` is a different representation: `source(file("events.jsonl"),
JsonLines)` is a `Seq<Data>` and `source(stdin, JsonLines)` a `Stream<Data>`.
`Json` and `JsonLines` are not aliases for output types; they express
different representation and framing. The same holds for `Csv`, `Markdown`,
`Hmd` and for the binary and record-oriented formats to come, `Avro`,
`Protobuf`, `Parquet`, which is why `Json : String` was the wrong
abstraction.

### The encoded representation in memory

`format(x, Json)` and its shorthands `json`, `yaml`, `csv`, `markdown`,
`text`, `hql`, `hmd`, `d2`, `mermaid` apply the encoder for `x`'s type in the
format and give the encoded text: a `String`, the one in-memory representation
of text HQL has. The value keeps its format at run time so that a host prints
it as it is and `| markdown` fences D2 and Mermaid source, but no program
reads `Json` as a type of a value.

### The I/O vocabulary stays

`source` introduces values into a flow, `sink` consumes a flow and writes it,
`tap` observes it and lets it continue:

```hql
source(file("input.txt"), Lines)
| tap(stderr, Lines)
| reverse
| sink(file("output.txt"), Lines)
```

`target`, which writes one value as one document, stays while HQL-0021 is
drafted; the direction is that `sink` covers a single value as one document
too and `target` is not needed beside it.

## Current state

- `Format` is in the lattice; every format type narrows it and none narrows
  `String`. `Lines` and `JsonLines` are formats. `format(x, F)` returns
  `String`, and `Stream<String>` over a stream.
- `Lines` and `Text` both read a file line by line today, and both write one
  value per line; whether `Text` becomes one whole document is a question for
  HQL-0021 when it is accepted.
- The decoder table is `io::document_type` and the encoder table is the
  renderer table of the [presentation card](../../../wiki/hql/presentation.hmd),
  both in Rust; `std/format.hql` declares `Decoder<F, T>` and `Encoder<T, F>`
  as the contracts they implement. A declared `impl Encoder<T, F> for …`
  written in HQL is not yet admitted: generic traits wait for a later record.
- [HQL-0017](../../HQL-0017/README.md) is superseded in its one rule that a
  format is a `String` subtype; its three questions and the renderer model
  stand.

## Work units

1. `Format`, the format types under it, `Lines` and `JsonLines`, and
   `format` returning `String`. Done.
2. The codec contracts in `std/format.hql`, with the Rust tables as their
   implementations and a test that every format has a codec. Done.
3. `target` folded into `sink`, and `Text` as one whole document, with
   HQL-0021.
4. Binary formats as a format kind, and codecs an extension or an HQL `impl`
   provides, once generic traits exist.
