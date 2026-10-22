# 13. Reading and writing data

!!! note "The `io` extension"

    This chapter covers the `io` extension, which ships in the next release.
    It is part of the prelude, so a program needs no `import` to use it.

Every program so far read a vault, or values written into the program itself.
This chapter reads files and standard input, and writes files and standard
output, without leaving the language. The pipeline you write stays the same
whatever the data's location.

## Endpoints

An **endpoint** says where bytes are. Constructing one performs no I/O: it
opens, stats and resolves nothing.

<!-- run: skip -->
```hql
file("examples/io/sightings.jsonl")
```

```text
file("examples/io/sightings.jsonl")
```

Four endpoints are bound by the host, as `vault` is, and need no configuration:
`stdin`, `stdout`, `stderr` and `file(path)`. A relative path resolves against
the directory of the program file or document that names it, and against the
working directory for an expression.

What an endpoint can do is its type. `Stdin` is `Readable`, `Stdout` and
`Stderr` are `Writable`, and `File` is both, and `Replayable`: it gives the
same bytes on every read within one evaluation. Reading from an endpoint that
cannot be read is refused before the program runs:

<!-- run: skip -->
```hql
source(stdout, Json)
```

```text
type error: `source` needs Readable, not Stdout
 --> <expression>:1:1
  |
1 | source(stdout, Json)
  | ^^^^^^^^^^^^^^^^^^^^
```

## Reading with `source`

`source(endpoint, Format)` reads every document an endpoint holds, in a
format. `examples/io/sightings.jsonl` holds four JSON documents, one per line:

```text
{"species": "barn owl", "site": "meadow", "seen": "dusk"}
{"species": "wren", "site": "hedge", "seen": "morning"}
{"species": "barn owl", "site": "barn", "seen": "night"}
{"species": "kingfisher", "site": "river", "seen": "morning"}
```

A source is an ordinary value, and a pipeline starts from it the way it starts
from `cards`:

<!-- run: skip -->
```hql
source(file("examples/io/sightings.jsonl"), Json)
| filter(s => s.seen == "morning")
| map(s => s.species)
```

```text
["wren", "kingfisher"]
```

A literal path whose extension names a format — `.json`, `.jsonl`, `.yaml`,
`.csv`, `.txt` — implies it, so the format may be left out:

<!-- run: skip -->
```hql
source(file("examples/io/sightings.jsonl"))
| group(by = s => s.species)
| map(g => {species: g.key, sightings: g.items | size})
```

```text
{{sightings: 2, species: "barn owl"}, {sightings: 1, species: "wren"}, {sightings: 1, species: "kingfisher"}}
```

Each document is `Data`, the type of a dynamically structured value from
[chapter 2](02-data.md). A CSV row is a record of strings, keyed by the
header.

## Finite or live: the type decides

A file is replayable, so reading it gives a `Seq`: it ends, and every step in
the book applies. Standard input arrives once, as it is written, so reading it
gives a `Stream`. `hql check` prints each type:

<!-- run: skip -->
```hql
source(file("examples/io/sightings.jsonl"))
```

```text
Seq<Data>
```

<!-- run: skip -->
```hql
source(stdin, Json)
```

```text
Stream<Data>
```

That difference is the one [chapter 9](09-streams.md) drew. `map`, `filter` and
`take` run over standard input one document at a time, as it arrives, and a
step that needs the whole input is refused until the stream is bounded or
batched. The same program works on a file and on a pipe, and the checker says
which steps each one allows.

`tap(endpoint, Format)` reads one document at a time from any readable
endpoint, a file included, holding only the current one in memory. Over a
file's end, `file(p).end`, it follows what is appended, as `tail -F` does:

<!-- run: skip -->
```hql
tap(file("app.log").end, Lines) | take(1)
```

waits for the next line written to `app.log`, and returns it.

## Writing with `target` and `sink`

Writing has two forms, chosen by the shape of the value.

`target(endpoint, Format)` writes one completed value as one document. Over a
file it replaces the file, atomically:

<!-- run: skip -->
```hql
source(file("examples/io/sightings.jsonl"))
| map(s => {species: s.species, site: s.site})
| target(file("sites.csv"))
```

```text
site,species
meadow,barn owl
hedge,wren
barn,barn owl
river,kingfisher
```

`sink(endpoint, Format)` writes one document per value, as each arrives. It is
how a stream is written, because a stream may never complete:

<!-- run: skip -->
```hql
source(stdin, Json)
| filter(s => s.seen == "morning")
| map(s => s.species)
| sink(stdout, Text)
```

```text
"wren"
"kingfisher"
```

Asking `target` to write a stream as one document is refused, with the two
ways out:

<!-- run: skip -->
```hql
source(stdin, Json) | target(file("all.json"))
```

```text
type error: `target` writes one completed value, and Stream<Data> may not end; write it piece by piece with `sink`, or bound it with `take(n)` first
 --> <expression>:1:23
  |
1 | source(stdin, Json) | target(file("all.json"))
  |                       ^^^^^^^^^^^^^^^^^^^^^^^^
```

Writing `file(p)` replaces it; writing `file(p).end` appends. One evaluation
may replace a file once or append to it any number of times, never both.

Both steps return `Unit`, and a `Unit` result prints nothing. What each one
accepts, by the shape of the values:

| | `Seq<T>` | `Stream<T>` |
| --- | --- | --- |
| `source` | from a replayable endpoint | from a live endpoint |
| `target` | one document | refused: `collect(n)`, or `take(n)` |
| `sink` | one document per element | one document per value |

Batches are streams of sequences, so `collect(100) | sink(stdout, Json)` writes
one JSON array per hundred values. The format describes the encoding, and the
type describes the shape: JSON lines is not a separate format, but `Json`
written by `sink`.

## Branches that write

A `tee` branch from [chapter 5](05-questions.md) may end in a sink. The branch
writes its documents, and the main pipeline goes on with every value:

<!-- run: skip -->
```hql
source(stdin, Json)
| tee archived =
    filter(e => e.level == "error")
    | sink(file("errors.jsonl").end, Json)
| map(e => e.msg)
| sink(stdout, Text)
```

One program reads a log as it is written, appends its errors to a file, and
prints every message. There is no orchestration around the query: the query
is the dataflow.

## On the command line

With these steps `hql` is a structured-data tool for the shell. A first
argument that names no command is a program, so `hql '<program>'` means
`hql eval '<program>'`:

```text
$ cat examples/io/sightings.jsonl | hql 'source(stdin, Json) | filter(s => s.seen == "morning") | map(s => s.species)'
"wren"
"kingfisher"
```

`--to json` pretty-prints each streamed document, as `jq .` does, and
`--compact` gives one line each:

```text
$ head -2 examples/io/sightings.jsonl | hql 'source(stdin, Json)' --to json --compact
{"seen":"dusk","site":"meadow","species":"barn owl"}
{"seen":"morning","site":"hedge","species":"wren"}
```

Converting between formats is a source and a `--to`:

```text
$ hql 'source(file("examples/io/sightings.jsonl")) | map(s => s.site)' --to yaml
- meadow
- hedge
- barn
- river
```

Input and output stay in the program; `--to` only chooses how the CLI writes
the final result.

## Effects and documents

Constructing an endpoint is pure. `source`, `tap`, `target` and `sink` are
**effectful**: they reach outside the program, and the host decides which of
them it allows.

`hql render` is one such host. A document reads only files under its own
directory, reads no standard input, and writes nothing outside itself. A cell
that tries is refused when the document is checked, before any cell runs:

```text
type error: a document writes nothing outside itself, so `target` and `sink` are refused here
 --> report.hmd:8:7
  |
8 | [1] | target(file("out.json"))
  |       ^^^^^^^^^^^^^^^^^^^^^^^^
```

The same program runs under different capability policies without changing
its model: a terminal may write files, and a document being rendered may not.

## What is next

Not every endpoint is bytes. [Chapter 14](14-channels-and-logs.md) passes
values between parts of a program through channels, and logs what a program
does.
