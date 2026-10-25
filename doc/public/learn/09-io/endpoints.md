# Endpoints

An **endpoint** states where bytes are. Constructing one performs no I/O: it
opens, stats and resolves nothing, and prints as it was written:

```hql
file("examples/io/sightings.jsonl")
```

```text
file("examples/io/sightings.jsonl")
```

Four endpoints are bound by the host, as `vault` is, and need no
configuration: `stdin`, `stdout`, `stderr` and `file(path)`.

## What an endpoint can do is its type

An endpoint's capabilities are its supertypes. A `Readable` endpoint is read
by `source` and `tap`, a `Writable` one is written by `target` and `sink`, and
a `Replayable` endpoint is a `Readable` one that gives the same bytes on every
read within one evaluation. `Stdin` is `Readable`, `Stdout` and `Stderr` are
`Writable`, and `File` is `Replayable` and `Writable`. Reading an endpoint
that is not readable is a type error, reported before the program runs:

```hql
source(stdout, Json)
```

```text
type error: `source` needs (Readable | String), not Stdout
 --> <expression>:1:1
  |
1 | source(stdout, Json)
  | ^^^^^^^^^^^^^^^^^^^^
```

The `String` in the diagnostic is the other form of `source`, a configured
source named by its id ([sources and targets by id](configured.md)).

## A file and its end

`file(p)` is the whole file. `file(p).end` is a second endpoint, the end of the
same file: `Readable` and `Writable`, and not `Replayable`, since what comes
after the end is not there yet. Which endpoint a step names decides what the
step does, so a file has no open mode:

| | `file(p)` | `file(p).end` |
| --- | --- | --- |
| `source` | every document in the file, as a `Seq` | a type error naming `tap` |
| `tap` | the documents one at a time, then it ends | the documents appended from now on, as `tail -f` gives them |
| `target`, `sink` | replace the file | append to the file |

```hql
source(file("app.log").end, Lines)
```

```text
type error: the end of a file holds nothing yet; follow what is appended there with `tap`
 --> <expression>:1:8
  |
1 | source(file("app.log").end, Lines)
  |        ^^^^^^^^^^^^^^^^^^^
```

## Where a relative path resolves

A relative path resolves against the directory of the source unit that names
it: the document for `hql render`, the program file for `hql run`, and the
working directory for `hql eval` and the REPL. A document or program reads the
same file wherever it is run from. The programs in this chapter run from the
repository root, so `examples/io/sightings.jsonl` is the file in the
[I/O examples](https://github.com/ewiger/hql/tree/main/examples/io).

A path with a scheme is a type error naming the network endpoint, since a URL
is not a file:

```hql
file("https://example.com/x.json")
```

```text
type error: "https://example.com/x.json" is a URL, not a file; a network resource is read with `http`
 --> <expression>:1:1
  |
1 | file("https://example.com/x.json")
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

[The network](network.md) adds `http`, `websocket` and `socket` with
`import net`, and a [channel](channels.md) is an endpoint of values rather
than bytes.
