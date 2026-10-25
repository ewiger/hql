# 14. I/O

Every program so far read a vault, or values written into the program itself.
This chapter reads files, standard input and the network, writes files and
standard output, passes values between parts of a program, and logs what a
program does, all through one model: a program reads from a **source** and
writes to a **target**.

## Sources and targets

`source(endpoint, Format)` reads every document an endpoint holds and gives
them as values. A source is a value, and a pipeline starts from it the way it
starts from `cards`:

```hql
source(file("examples/io/sightings.jsonl"), Json)
| filter(s => s.seen == "morning")
| map(s => s.species)
```

```text
["wren", "kingfisher"]
```

Three things meet in that first step, and each has a page. An **endpoint**,
`file("examples/io/sightings.jsonl")`, states where bytes are; constructing it
performs no I/O. A **format**, `Json`, states how the bytes are documents,
here one JSON document per line. `source` joins the two. Whether the documents
end is carried by the type of what it gives: a file gives a `Seq`, standard
input a `Stream`, and the checker reports which steps each one allows.

Writing is the mirror. `target(endpoint, Format)` writes one completed value
as one document, and `sink(endpoint, Format)` writes one document per value
as each arrives:

```hql
source(file("examples/io/sightings.jsonl"))
| filter(s => s.seen == "morning")
| map(s => s.species)
| sink(stdout, Text)
```

```text
wren
kingfisher
```

Both writing steps give `Unit`, and a `Unit` result prints nothing: the two
lines above were written by `sink` while the program ran.

The `io` module is part of the prelude, so none of this needs an `import`.
Logging and the network are modules of their own, `import logging` and
`import net`.

## The pages

| Page | What it covers |
| --- | --- |
| [Endpoints](endpoints.md) | `stdin`, `stdout`, `stderr`, `file(p)` and `file(p).end`; `Readable`, `Writable` and `Replayable`; where a relative path resolves |
| [Reading with `source`](source.md) | documents as `Data`, the format from the extension, reading as a declared record, and the decode error |
| [Writing with `target` and `sink`](target.md) | one document or one document per value, replace or append, and branches that write |
| [Finite or live](finite-or-live.md) | `Seq` from a file and `Stream` from a pipe, `tap`, and batches with `collect(n)` |
| [Formats and documents](formats.md) | what one document reads as, how documents are separated, and which extension implies which format |
| [Sources and targets by id](configured.md) | `[sources.<id>]` in `hql.toml`, the kind that states the type, the schema boundary, and writing by id |
| [Channels](channels.md) | `Channel<T>(name)`, an endpoint of values held in memory |
| [Logging](logging.md) | `Logger`, the five levels, `--log-level`, and logs kept as values |
| [The network](network.md) | `http`, `websocket` and `socket` with `import net` |
| [Effects and what a host allows](hosts.md) | effectful operations, and what `hql eval`, `hql render` and `hql check` grant |
| [`hql` on the command line](command-line.md) | `hql '<program>'`, `--to`, `--compact` and `--from` |
