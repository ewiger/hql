# Formats and documents

A **format** states how bytes are documents: how one document is read from
bytes and written to them, and how one document is separated from the next.
The formats are the `Format` types of the
[writing results out chapter](../11-output.md). `source` and `tap` decode
with them, `target` and `sink` encode with them.

## What one document reads as

| Format | One document reads as | Documents are separated by |
| --- | --- | --- |
| `Json` | `Data` | whitespace between top-level values |
| `Yaml` | `Data` | a `---` line |
| `Csv` | `Data`, one row keyed by the header | a line; the header is read once |
| `Lines` | `String`, one line without its terminator | a line terminator |
| `JsonLines` | `Data`, one JSON document per line | a line terminator |
| `Text`, `Markdown`, `Hmd`, `Hql` | `String`, the whole text | not separable: one document |

`Lines` and `JsonLines` are framing formats, one document per line. The same
file read as `Lines` gives its lines as strings, and read as `Json` or
`JsonLines` the documents those lines hold:

```hql
source(file("examples/io/sightings.jsonl"), Lines) | take(1)
```

```text
["{\"species\": \"barn owl\", \"site\": \"meadow\", \"seen\": \"dusk\"}"]
```

A whole-text format reads one document, the whole file:

```hql
source(file("examples/io/counts.csv"), Text) | size
```

```text
1
```

Bytes that do not decode in their format are an evaluation failure naming the
endpoint, the line and the column.

`D2` and `Mermaid` describe a whole, so they are written by `target` only:

```hql
source(file("examples/io/sightings.jsonl"), Mermaid)
```

```text
type error: Mermaid is written, not read
 --> <expression>:1:8
  |
1 | source(file("examples/io/sightings.jsonl"), Mermaid)
  |        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

```hql
source(file("examples/io/sightings.jsonl")) | sink(stdout, Mermaid)
```

```text
type error: Mermaid describes a whole, so it is written by `target`, not `sink`
 --> <expression>:1:47
  |
1 | source(file("examples/io/sightings.jsonl")) | sink(stdout, Mermaid)
  |                                               ^^^^^^^^^^^^^^^^^^^^^
```

## The extension implies the format

When the format argument is left out, the extension of a literal file path
implies it:

| Extension | Format |
| --- | --- |
| `.json` | `Json` |
| `.jsonl`, `.ndjson` | `JsonLines` |
| `.yaml`, `.yml` | `Yaml` |
| `.csv` | `Csv` |
| `.txt`, `.log` | `Lines` |
| `.md` | `Markdown` |
| `.hmd` | `Hmd` |
| `.hql` | `Hql` |
| `.d2` | `D2` |
| `.mmd` | `Mermaid` |

A format that is given wins over the extension: `source(file("notes.data"),
Json)` and `target(file("result.txt"), Json)` are both allowed. An extension
outside the table implies none, and leaving the format out is then a type
error naming the argument:

```hql
source(file("examples/io/notes.data"))
```

```text
type error: the extension of "examples/io/notes.data" implies no format; name it: `source(…, Json)`
 --> <expression>:1:8
  |
1 | source(file("examples/io/notes.data"))
  |        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

## The format is not the shape

A format states the encoding, and the type states the shape. A `Seq` written
by `target` in `Json` is one array; written by `sink` it is one document per
line, which `Json` reads back, since whitespace separates top-level JSON
values. `sink(e, F)` and `source(e, F)` are inverses: the documents one
writes are the documents the other reads, in order.

Reading in one format and writing in another converts a file. A CSV cell is
text, so the counts arrive in YAML as strings:

```hql
source(file("examples/io/counts.csv")) | target(stdout, Yaml)
```

```text
- count: '2'
  species: barn owl
- count: '1'
  species: wren
- count: '1'
  species: kingfisher
```

Read as a declared record, the same cells are the `Int` the record declares
([reading with `source`](source.md)).
