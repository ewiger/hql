# Writing with `target` and `sink`

Writing has two forms, chosen by the shape of the value. `target(endpoint,
Format)` writes one completed value as one document. `sink(endpoint, Format)`
writes one document per value, as each arrives. Both give `Unit`, and a
`Unit` result prints nothing, so what a program below prints is what the
step wrote.

## One document with `target`

```hql
source(file("examples/io/sightings.jsonl"))
| map(s => {species: s.species, site: s.site})
| target(stdout, Csv)
```

```text
site,species
meadow,barn owl
hedge,wren
barn,barn owl
river,kingfisher
```

A `Seq` written by `target` is one document: in `Csv` a header and a row per
element, in `Json` one array:

```hql
source(file("examples/io/sightings.jsonl")) | map(s => s.species) | target(stdout, Json)
```

```text
[
  "barn owl",
  "wren",
  "barn owl",
  "kingfisher"
]
```

Over a file, `target` replaces the file atomically: the document is written
beside the destination and renamed over it when complete, so a reader never
sees a partial file, and a failed write leaves the old file in place. Run
from a directory of your own:

<!-- run: skip -->
```hql
source(file("examples/io/sightings.jsonl"))
| map(s => {species: s.species, site: s.site})
| target(file("sites.csv"))
```

```text
$ cat sites.csv
site,species
meadow,barn owl
hedge,wren
barn,barn owl
river,kingfisher
```

## One document per value with `sink`

```hql
source(file("examples/io/sightings.jsonl")) | map(s => s.species) | sink(stdout, Json)
```

```text
"barn owl"
"wren"
"barn owl"
"kingfisher"
```

A `Seq` written by `sink` in `Json` is one JSON document per line, which is
what `JsonLines` and `Json` both read back. A stream may never complete, so
`sink` is how a stream is written. `target` over a stream that is not a `Seq`
is a type error, and the diagnostic gives the two ways out:

```hql
source(stdin, Json) | target(file("all.json"))
```

```text
check error[type-mismatch]: `target` writes one completed value, and Stream<Data> may not end; write it piece by piece with `sink`, or bound it with `take(n)` first
 --> <expression>:1:23
  |
1 | source(stdin, Json) | target(file("all.json"))
  |                       ^^^^^^^^^^^^^^^^^^^^^^^^
```

What each step accepts, by the shape of the values:

| | `Seq<T>` | `Stream<T>` that is not a `Seq` |
| --- | --- | --- |
| `target` | one document | a type error: `sink`, or `take(n)` first |
| `sink` | one document per element | one document per value |

A batch is a list, so `collect(2) | sink(stdout, Json)` writes one JSON array
per two values ([finite or live](finite-or-live.md)).

## Replace or append

Writing `file(p)` replaces the file. Writing `file(p).end` appends to it, and
creates it when it is missing. Run twice, once with each species:

<!-- run: skip -->
```hql
[{species: "wren"}] | sink(file("log.jsonl").end)
```

```text
$ cat log.jsonl
{"species":"wren"}
{"species":"kite"}
```

`sink` over `file(p)` truncates the file before its first document and writes
each document as it arrives, since a stream that never ends cannot be renamed
into place.

One evaluation replaces a file at most once, appends to its end any number of
times, and never both. Reading a file the same evaluation has replaced is an
evaluation failure too, as is a missing parent directory: neither step
creates a directory.

<!-- run: skip -->
```hql
1 | target(file("a.json"))
2 | target(file("a.json"))
```

```text
evaluation failure: a.json: this evaluation has already replaced the file; append to `file(p).end`, or write it once
 --> <expression>:2:5
  |
2 | 2 | target(file("a.json"))
  |     ^^^^^^^^^^^^^^^^^^^^^^
```

## Branches that write

A `tee` branch from the [SQL-like queries chapter](../06-sql-like-queries.md)
may end in `target` or `sink`. The branch receives each value before the
trunk does and writes it, and the trunk goes on with every value:

<!-- run: skip -->
```hql
source(stdin, Json)
| tee dusk =
    filter(s => s.seen == "dusk")
    | sink(file("dusk.jsonl").end, Json)
| map(s => s.species)
| sink(stdout, Text)
```

```text
$ cat examples/io/sightings.jsonl | hql '…'
barn owl
wren
barn owl
kingfisher
$ cat dusk.jsonl
{"seen":"dusk","site":"meadow","species":"barn owl"}
```

One program reads sightings as they arrive, appends those seen at dusk to a
file, and prints every species. A failed write on any branch ends the
evaluation with that error.

## Standard output and the result

`target(stdout, F)` and `sink(stdout, F)` write while the program runs. The
result of a program is a different thing: the host presents it after
evaluation, in the format `--to` names
([`hql` on the command line](command-line.md)). A program that ends in
`target` or `sink` has the result `Unit`, and prints nothing beyond what it
wrote.
