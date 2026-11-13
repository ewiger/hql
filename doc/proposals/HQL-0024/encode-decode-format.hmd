The model we converged on is roughly this.

`Format` is a real language-level category. It is **not** `Format<T>`, and formats are not subtypes of the values they encode or decode.

For example:

```hql
type Format

Text      : Format
Lines     : Format
Json      : Format
JsonLines : Format
Yaml      : Format
Csv       : Format
```

The important point is that `Lines` is **not**:

```hql
Lines : String
```

and not:

```hql
Lines : Seq<String>
```

because that would mix up **representation** with **data**.

A format describes an external representation and, where applicable, its framing rules.

Conceptually:

```text
bytes
  |
  v
Format
  |
  +-- Decoder
  |
  v
HQL values
```

and in the opposite direction:

```text
HQL values
  |
  +-- Encoder
  |
  v
Format
  |
  v
bytes
```

The typed relationship can be expressed through contracts conceptually similar to:

```hql
trait Decoder<F : Format, T>
trait Encoder<T, F : Format>
```

The exact syntax does not have to be this, but the important semantic relation is:

```text
Format + Decoder -> HQL value
HQL value + Encoder + Format -> encoded representation
```

For example:

```text
Decoder<Lines, String>
Encoder<String, Lines>
```

means that `Lines` can decode logical line records into `String` values and encode `String` values back into line records.

The format itself does not contain the implementation.

The HQL layer defines the contract. The extension declares that it implements the contract. The actual codec implementation can live in Rust.

So the architecture is:

```text
HQL declaration / contract
          |
          v
Extension contract
          |
          v
Rust implementation
    - parsing
    - encoding
    - framing
    - buffering
    - low-level IO mechanics
```

This fits the broader HQL module/extension philosophy: the public interface is described in HQL, while native implementation details remain inside the extension.

## Framing

`Lines` demonstrates why format and value type must remain separate.

Given:

```text
foo\nbar\nbaz\n
```

reading through `Lines` should logically produce:

```hql
["foo", "bar", "baz"]
```

not:

```hql
["foo\n", "bar\n", "baz\n"]
```

The newline belongs to the **representation/framing**, not to the resulting `String`.

Likewise, when writing:

```hql
["foo", "bar", "baz"]
```

through `Lines`, the encoder/framer is responsible for producing the required line separators.

Conceptually:

```text
Lines
    =
record framing
    +
text decoding/encoding rules
```

The details such as:

```text
\n
\r\n
UTF-8 decoding
buffering partial records
final unterminated line
```

belong to the codec/framing implementation, not to ordinary HQL programs.

The programmer should not normally need to write:

```hql
split("\n")
```

just to process a file as lines.

## Format does not determine cardinality

A second important distinction is between **representation** and **source topology/cardinality**.

For example:

```hql
source(file("input.txt"), Lines)
```

can naturally produce:

```hql
Seq<String>
```

because a file is finite.

But:

```hql
source(stdin, Lines)
```

naturally produces:

```hql
Stream<String>
```

because stdin is a streaming source.

Therefore `Lines` itself means neither:

```hql
Seq<String>
```

nor:

```hql
Stream<String>
```

The source determines the execution shape.

Conceptually:

```text
file + Lines
    |
    v
finite decoded records
    |
    v
Seq<String>
```

while:

```text
stdin + Lines
    |
    v
incrementally decoded records
    |
    v
Stream<String>
```

The separation is:

```text
Source
    transport
    bounded/unbounded behavior
    execution topology/cardinality

Format
    external representation
    framing semantics

Decoder
    representation -> HQL values

Encoder
    HQL values -> representation
```

The codec therefore does **not** decide whether the final HQL result is `Seq<T>` or `Stream<T>`.

That comes from the source/sink execution model.

## JSON illustrates the same distinction

For example:

```hql
source(file("data.json"), Json)
```

can decode one JSON document into:

```hql
Data
```

But line-oriented JSON is a different representation:

```hql
JsonLines : Format
```

and:

```hql
source(file("events.jsonl"), JsonLines)
```

could produce:

```hql
Seq<Data>
```

while:

```hql
source(stdin, JsonLines)
```

could produce:

```hql
Stream<Data>
```

`Json` and `JsonLines` therefore are not aliases for output types. They express different representation and framing semantics.

The same applies to future formats such as:

```text
Csv
Avro
Protobuf
Parquet
Markdown
Hmd
```

Some are textual, some binary, some record-oriented, some document-oriented.

This is another reason why:

```hql
Json : String
```

is the wrong abstraction.

## IO vocabulary

The current IO vocabulary should remain:

```text
source
sink
tap
```

`source` introduces values into a flow.

`sink` consumes the flow and writes it somewhere.

`tap` performs a side observation/output while allowing the original flow to continue.

For example:

```hql
source(file("input.txt"), Lines)
| reverse
| sink(stdout, Lines)
```

or:

```hql
source(file("input.txt"), Lines)
| tap(stderr, Lines)
| reverse
| sink(file("output.txt"), Lines)
```

There is no need for a parallel `target(...)` concept if `sink(...)