# Finite or live

A file is replayable, so reading it gives a `Seq`: the documents end, and
every step in the book applies. Standard input arrives once, as it is
written, so reading it gives a `Stream`. `hql check` prints each type:

<!-- run: check -->
```hql
source(file("examples/io/sightings.jsonl"))
```

```text
Seq<Data>
```

<!-- run: check -->
```hql
source(stdin, Json)
```

```text
Stream<Data>
```

That is the difference the [streams chapter](../07-streams.md) drew. `map`,
`filter` and `take` run over standard input one document at a time, as it
arrives. A step that needs the whole input is a type error until the stream is
bounded with `take(n)` or batched with `collect(n)`:

```hql
source(stdin, Json) | size
```

```text
type error: `size` needs a finite input, and Stream<Data> may not end; bound it first with `take(n)`, or batch it with `collect(n)`
 --> <expression>:1:23
  |
1 | source(stdin, Json) | size
  |                       ^^^^
```

The same program checks over a file and over a pipe, and the checker reports
which steps each one allows.

## One document at a time with `tap`

`tap(endpoint, Format)` reads any readable endpoint as a `Stream`, one
document at a time, holding only the current one in memory. Over a file,
`source` holds every document and gives a `Seq`; `tap` holds one and gives a
`Stream`, so a file larger than memory is read with `tap`:

<!-- run: check -->
```hql
tap(file("examples/io/sightings.jsonl"))
```

```text
Stream<Data>
```

```hql
tap(file("examples/io/sightings.jsonl")) | take(2) | map(s => s.species)
```

```text
["barn owl", "wren"]
```

Over a live endpoint the two give the same stream: `tap(stdin, Json)` is
`source(stdin, Json)`. Over a file's end, `tap` gives each document appended
after the tap began, as `tail -F` does, and waits for a file that does not
exist yet:

<!-- run: skip -->
```hql
tap(file("app.log").end, Lines) | take(1)
```

waits for the next line written to `app.log`, and gives it.

A format that is one whole document, such as `Text`, is not read one at a
time:

```hql
tap(file("examples/io/counts.csv"), Text)
```

```text
type error: Text is one document, so it is not read one at a time; read it whole with `source`
 --> <expression>:1:5
  |
1 | tap(file("examples/io/counts.csv"), Text)
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

## Batches

`collect(n)` gathers consecutive values into lists of `n`, and a list is
finite, so a stream is answered in pieces:

```hql
tap(file("examples/io/sightings.jsonl")) | collect(2) | map(b => b | size) | take(2)
```

```text
[2, 2]
```

Written by `sink`, each batch is one document:

```hql
tap(file("examples/io/sightings.jsonl")) | map(s => s.species) | collect(2) | sink(stdout, Json)
```

```text
["barn owl","wren"]
["barn owl","kingfisher"]
```

Over standard input the same program writes each array as soon as its second
value has arrived:

```text
$ cat examples/io/sightings.jsonl | hql 'source(stdin, Json) | map(s => s.species) | collect(2) | sink(stdout, Json)'
["barn owl","wren"]
["barn owl","kingfisher"]
```

## What each shape allows

A `Seq<T>` is a `Stream<T>` that ends, so every rule for a stream holds for a
sequence, and a sequence has more:

| | `Seq<T>` | `Stream<T>` that is not a `Seq` |
| --- | --- | --- |
| `source` gives it | over a `Replayable` endpoint or a channel | over a live endpoint |
| `tap` | reads it one value at a time | reads it as it is |
| `target` | writes one document | a type error |
| `sink` | writes one document per element | writes one document per value |
| the result, by `--to` | one whole text | one piece per value, as it arrives |
| `size`, `sort`, `xs[i]`, `contains` | accepted | a type error |
| `filter`, `map`, `take`, `drop`, `tee` | accepted | accepted, one value at a time |
| `collect(n)` | gives `List<List<T>>` | gives `Stream<List<T>>`, in batches |

`emit` keeps its meaning from the streams chapter: it gives the members of
each list a stream carries. It is not needed to turn a `Seq` into a `Stream`,
because a `Seq` already is one.
