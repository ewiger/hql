# Effects and what a host allows

Constructing an endpoint, a channel or a logger is pure: it names a place and
touches nothing. `source`, `tap`, `target`, `sink` and every log call are
**effectful**: they reach outside the program. A function whose body uses an
effectful operation, or calls an effectful function, is effectful too, and
`hql check` reports each one with what it uses, before the type of the
program:

<!-- run: check -->
```hql
fn load() : Seq<Data> = source(file("examples/io/sightings.jsonl"))
load() | size
```

```text
load is Effectful: it uses `source`
Int
```

Every effect is known before evaluation, so the host checks each one against
its policy and reports a use it does not grant as a type error naming the
endpoint:

| Host | Reads | Writes |
| --- | --- | --- |
| `hql eval`, `hql run`, `hql repl` | any endpoint | any endpoint |
| `hql render` | files under the document's directory, and channels | channels |
| `hql check` | none | none |

`hql check` reads nothing but the program, so checking is offline: a program
over a file that does not exist yet, or over the network, still checks.

## A document reads beside itself and writes only its channels

`hql render` evaluates the cells of a document
([queries inside documents](../19-documents.md)). A document someone else
wrote changes no file and sends nothing: a cell may read files under the
document's own directory and write channels, and nothing else. A cell that
writes a file, standard output or standard error is reported when the
document is checked, before any cell runs:

````markdown
# Report

```hql
[1] | target(file("out.json"))
```
````

```text
$ hql render report.hmd
type error: a document writes nothing outside itself, so `target` and `sink` are refused here
 --> report.hmd:4:7
  |
4 | [1] | target(file("out.json"))
  |       ^^^^^^^^^^^^^^^^^^^^^^^^
```

A network endpoint in a document is a type error in the same way, and a file
read that leaves the document's directory, by `..` or through a symbolic
link, is an evaluation failure naming the path:

```text
type error: a document reads and writes nothing on the network
 --> net.hmd:5:8
  |
5 | source(http("https://example.com/x"), Json)
  |        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

```text
evaluation failure: ../outside.json: a document reads only files under its own directory
 --> outside.hmd:4:1
  |
4 | source(file("../outside.json"))
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

A logger is checked as the `sink` it is, so a cell logs to `cell.logs`, its
own channel, and not to `stderr` ([logging](logging.md)). The same program
runs under different policies without changing its model: a terminal may
write files, and a document being rendered may not.

## Bounds

A `source` over a replayable endpoint holds every document, and the
configuration's `max_bytes` bounds it; `tap` holds one document at a time and
is bounded by what consumes it. A channel holds what its scope wrote, up to a
million values; one more ends the evaluation, naming the channel.
