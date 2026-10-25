<!-- hql reference: std/io -->
# `std/io`

```hql
module io
```

Input and output: endpoints, the data sources read from them, and the
data targets written to them. An endpoint names where bytes are and
performs no I/O; a format type (module `format`) states how the bytes are
documents; a source or target joins the two, and `Seq` and `Stream` state
whether the documents end.

- [`Endpoint`](types/Endpoint.md) — The abstract type of every endpoint: where bytes are.
- [`Readable`](types/Readable.md) — An endpoint bytes can be read from, by `source` and `tap`.
- [`Replayable`](types/Replayable.md) — A `Readable` endpoint that gives the same bytes on every read in one evaluation, so `source` reads what it holds as a `Seq`.
- [`Writable`](types/Writable.md) — An endpoint bytes can be written to, by `target` and `sink`.
- [`Stdin`](types/Stdin.md) — The program's standard input, bound as `stdin`: read once, as it arrives.
- [`Stdout`](types/Stdout.md) — The program's standard output, bound as `stdout`: written while the program runs, before the host presents its result.
- [`Stderr`](types/Stderr.md) — The program's standard error, bound as `stderr`: where diagnostics go, apart from the result.
- [`FileEnd`](types/FileEnd.md) — The end of a file, after what it holds, as `file(path).end`: a write there appends, and `tap` follows what is written next, as `tail -f` does.
- [`File`](types/File.md) — A local file, the whole of it: read, every document it holds; written, it is replaced.
- [`Channel`](types/Channel.md) — An endpoint of values rather than bytes, held in memory for one execution scope: a program run, or one cell.
- [`DataSource`](types/DataSource.md) — What a program reads: a configured source by id, or an endpoint read in a format.
- [`FiniteSource`](types/FiniteSource.md) — A source of every document, in order, that ends: read from a `Replayable` endpoint.
- [`LiveSource`](types/LiveSource.md) — A source of each document as it arrives, with no replay: read from a live endpoint.
- [`KeyedSource`](types/KeyedSource.md) — A finite source whose contents are addressable by a key of type `K`.
- [`Position`](types/Position.md) — A message's place in its log, ordered as the log is.
- [`Message`](types/Message.md) — One element of a stream with where and when it sits in it: its `value`, its `position`, an optional `key` and `time`, and its `headers`.
- [`StreamSource`](types/StreamSource.md) — A log: `Message<T>`s in order, replayable within its retention.
- [`Catalog`](types/Catalog.md) — An endpoint that contains sources of type `S`: a database of tables, a cluster of topics.
- [`VaultSource`](types/VaultSource.md) — A vault as a source: an endpoint that holds its cards, as a catalog holds tables, with its members as fields rather than names.
- [`DataTarget`](types/DataTarget.md) — What a program writes to: an endpoint written in a format, or a configured target by its id.
- [`file`](file.md) — The `File` at `path`.
- [`source`](source.md) — Every document read from `endpoint` in `format`, decoded to `T`: a `Seq` from a `Replayable` endpoint, a `Stream` from a live one.
- [`tap`](tap.md) — The documents read from `endpoint` in `format`, one held at a time, as a `Stream<T>`.
- [`target`](target.md) — Writes `value` as one document to `endpoint` in `format`, or to the target configured under `id`, and gives `Unit`.
- [`sink`](sink.md) — Writes each of `values` to `endpoint` in `format`, one document per value, and gives `Unit`: `[{a: 1}, {a: 2}] | sink(stdout, Json)` writes `{"a":1}` and `{"a":2}` on two lines.
<!-- hql reference: end -->
