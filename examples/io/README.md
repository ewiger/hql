# I/O: reading files as typed records

`sightings.jsonl` holds four JSON documents, one per line, and `counts.csv` a
header and three rows. The cases read them with `source`, from
[HQL-0021](../../doc/proposals/HQL-0021/README.md), and the
[I/O chapter](../../doc/public/learn/14-io/index.md)
reads the same files from the command line.

| Case | What it shows | Result |
| --- | --- | --- |
| [typed sightings](queries/typed-sightings.hql) | `s : Seq<Sighting> = source(file(…))`: each document decoded as the declared record | the species seen in the morning |
| [typed counts](queries/typed-counts.hql) | a CSV cell read as the `Int` its field declares | the sum of the counts |

From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/io/queries/typed-sightings.hql
target/debug/hql run examples/io/queries/typed-counts.hql
```

A relative path resolves against the program that names it, so both read
`../sightings.jsonl` and `../counts.csv` beside `queries/`.
