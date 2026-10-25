# Reading with `source`

`source(endpoint, Format)` reads every document an endpoint holds, in a
format, and gives them as values. `examples/io/sightings.jsonl` holds four
JSON documents, one per line:

```text
{"species": "barn owl", "site": "meadow", "seen": "dusk"}
{"species": "wren", "site": "hedge", "seen": "morning"}
{"species": "barn owl", "site": "barn", "seen": "night"}
{"species": "kingfisher", "site": "river", "seen": "morning"}
```

```hql
source(file("examples/io/sightings.jsonl"), Json)
| filter(s => s.seen == "morning")
| map(s => s.species)
```

```text
["wren", "kingfisher"]
```

## The format from the extension

A literal path whose extension names a format, `.json`, `.jsonl`, `.yaml`,
`.csv` or `.txt` among them, implies it, so the format may be left out:

```hql
source(file("examples/io/sightings.jsonl"))
| group(by = s => s.species)
| map(g => {species: g.key, sightings: g.items | size})
```

```text
{{sightings: 2, species: "barn owl"}, {sightings: 1, species: "wren"}, {sightings: 1, species: "kingfisher"}}
```

The format may be left out only for `file` of a literal path, because the
type of what is read depends on the format, and a path computed at run time
has no extension the checker can read:

```hql
p = "examples/io/sightings.jsonl"
source(file(p))
```

```text
type error: `source` needs a format, `source(…, Json)`: only a literal file path implies one
 --> <expression>:2:8
  |
2 | source(file(p))
  |        ^^^^^^^
```

[Formats and documents](formats.md) has the table of formats and the
extensions that imply them.

## Each document is `Data`

Each document is `Data`, the type of a dynamically structured value from the
[data types chapter](../02-data-types/data.md). A CSV row is a record of
strings, keyed by the header:

```hql
source(file("examples/io/counts.csv"))
```

```text
[{count: "2", species: "barn owl"}, {count: "1", species: "wren"}, {count: "1", species: "kingfisher"}]
```

A JSON array is one document, a `Data` that is a list. A file whose whole
content is one array holds one document, and its members are spread with
`of_type(List) | emit`:

```hql
source(file("examples/sources/station/data/sightings.json")) | size
```

```text
1
```

```hql
source(file("examples/sources/station/data/sightings.json"))
| of_type(List)
| emit
| map(s => s.species)
```

```text
["wren", "owl", "kite"]
```

A configured `json` source gives the members of its array without this step
([sources and targets by id](configured.md)).

## Reading as a declared record

A binding whose declared type names an element type decodes each document as
that element; so does a function whose declared result is the read:

```hql
type Sighting {
    species : String
    site    : String
    seen    : String
}

sightings : Seq<Sighting> = source(file("examples/io/sightings.jsonl"))
sightings | filter(s => s.seen == "morning") | map(s => s.species)
```

```text
["wren", "kingfisher"]
```

The same reading is written as a step, `source(file(p)) | Sighting`:

```hql
type Sighting {
    species : String
    site    : String
    seen    : String
}

source(file("examples/io/sightings.jsonl")) | Sighting | map(s => s.site)
```

```text
["meadow", "hedge", "barn", "river"]
```

A CSV cell is text, so it is read as the `Int`, `Float` or `Bool` its field
declares. A JSON or YAML leaf keeps the type it was written with: `"3"` is
not an `Int`.

```hql
type Tally {
    species : String
    count   : Int
}

tallies : Seq<Tally> = source(file("examples/io/counts.csv"))
tallies | map(t => t.count) | sum
```

```text
4
```

A document that does not decode is an evaluation failure naming the file,
the document, counted from 1, and the field:

```hql
type Sighting {
    species : String
    site    : String
    seen    : Int
}

sightings : Seq<Sighting> = source(file("examples/io/sightings.jsonl"))
sightings | size
```

```text
evaluation failure: examples/io/sightings.jsonl: document 1 does not decode as Sighting: field `seen`: "dusk" is not Int
 --> <expression>:7:29
  |
7 | sightings : Seq<Sighting> = source(file("examples/io/sightings.jsonl"))
  |                             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

A plain record is closed: a document with an entry the record does not
declare does not decode. A record narrowing `Data`, `type Sighting : Data
{ … }`, is an open view that keeps such entries beneath the fields it
declares:

```hql
type Sighting : Data {
    species : String
}

sightings : Seq<Sighting> = source(file("examples/io/sightings.jsonl"))
sightings | map(s => s.species)
```

```text
["barn owl", "wren", "barn owl", "kingfisher"]
```

What each kind of record reads and what it does not is
[open and closed types](../11-advanced-data-typing/open-and-closed.md) in the
advanced data typing chapter, and [data contracts](../11-advanced-data-typing/data-contracts.md)
add rules the values must hold.

A `source` bound to a name is read once per evaluation, however many times
the name is used.
