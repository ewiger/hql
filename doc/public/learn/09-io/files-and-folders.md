# Files and folders

A `file(p)` endpoint names one file by a path written in the program. The
`io.fs` module lists what a folder holds, so a program finds its files first
and then reads them. A program imports it, `import io.fs`, and reaches it as `fs`.

The programs on this page list a small tree in the
[file-system examples](https://github.com/ewiger/hql/tree/main/examples/fs):

```text
examples/fs/tree/
  assets/logo.txt        5 bytes
  assets/style.css      19 bytes
  data/2026/q1.csv      21 bytes
  data/2026-notes.md    17 bytes
  data/birds.csv        28 bytes
```

## A path

A **`Path`** is a place in a file system, written as text: segments separated
by `/`. It is part of `io`, so it needs no import. Its operations read the
text and never the file system, so they are pure:

```hql
p = Path("examples/fs/tree").join("data").join("birds.csv")
[p.text, p.name(), p.stem(), p.extension() | unwrap_or("?"), p.parent() | map(q => q.text) | unwrap_or("?")]
```

```text
["examples/fs/tree/data/birds.csv", "birds.csv", "birds", "csv", "examples/fs/tree/data"]
```

`parent()` is `none` for a path of one segment, and `extension()` is `none`
for a last segment with no `.`:

```hql
Path("data").parent()
```

```text
none
```

Paths are ordered by their segments, compared in turn, each by its text.
`2026/q1.csv` sorts before `2026-notes.md`, although `-` sorts before `/` as
text, since the first segments `2026` and `2026-notes.md` already differ:

```hql
[Path("data/2026-notes.md"), Path("data/2026/q1.csv")] | sort | map(p => p.text)
```

```text
["data/2026/q1.csv", "data/2026-notes.md"]
```

This is the order of a **sorted walk**, and every listing below gives it.

## A folder

`folder(path)` is the **`Folder`** at a path, a `String` or a `Path`. A folder
is an endpoint that holds files and folders, as a database holds tables.
Constructing one opens, stats and resolves nothing, and a relative path
resolves as a file's does ([where a relative path
resolves](endpoints.md#where-a-relative-path-resolves)):

```hql
import io.fs

folder("examples/fs/tree")
```

```text
Folder(path = Path(text = "examples/fs/tree"))
```

`io.fs` is not in the prelude. Without the import, `folder` is an unknown
name, reported before the program runs:

```hql
folder("examples/fs/tree")
```

```text
check error[unknown-name]: `folder` is provided by the `io.fs` extension: write `import io.fs`
 --> <expression>:1:1
  |
1 | folder("examples/fs/tree")
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^
```

A folder is not a `Seq` itself. Three effectful steps list it, each reading
the file system once, where it is written, and each giving a `Seq<Entry>`.
`entries` gives what is directly in the folder, sorted by name, an entry
whose name opens with `.` included:

```hql
import io.fs

folder("examples/fs/tree/data") | entries | map(e => [e.name, e.kind])
```

```text
[["2026", "folder"], ["2026-notes.md", "file"], ["birds.csv", "file"]]
```

`walk` gives every entry below the folder, the folder itself excluded: depth
first, each folder's entries sorted by name, and each folder followed at once
by what it holds:

```hql
import io.fs

folder("examples/fs/tree") | walk | map(e => e.path.text.trim_prefix("examples/fs/tree/"))
```

```text
["assets", "assets/logo.txt", "assets/style.css", "data", "data/2026", "data/2026/q1.csv", "data/2026-notes.md", "data/birds.csv"]
```

The order is never the file system's: the same tree walks the same way on
every machine. It is also the order a [vault](../21-a-vault.md) reads its
cards in; the vault skips entries whose name opens with `.`, and `walk` lists
them.

## An entry

An **`Entry`** is one thing a folder holds. It has five fields:

| Field | Type | What it holds |
| --- | --- | --- |
| `path` | `Path` | the listed folder's path joined with the entry's path below it |
| `name` | `String` | the last segment |
| `kind` | `String` | `"file"`, `"folder"` or `"link"` |
| `size` | `Int`, optional | a file's length in bytes; absent for a folder or a link |
| `modified` | `Instant`, optional | the time the entry was last modified, a link's own and not its target's; absent where the platform records none |

```hql
import io.fs

folder("examples/fs/tree")
| walk
| filter(e => e.name == "birds.csv")
| map(e => [e.path.text, e.kind, str(e.size)])
```

```text
[["examples/fs/tree/data/birds.csv", "file", "28"]]
```

`modified` is an [`Instant`](../02-data-types/time.md), so it compares with an
instant the program writes and sorts the entries by time. A listing reads it
once, with the rest of the entry, so the files changed since a cut-off are a
filter over it:

```hql
import io.fs

cutoff = Instant("2020-01-01T00:00:00Z")
folder("examples/fs/tree/data")
| entries
| filter(e => e.modified | map(t => t >= cutoff) | unwrap_or(false))
| map(e => e.name)
```

```text
["2026", "2026-notes.md", "birds.csv"]
```

The times themselves are the file system's, so they differ from one checkout
of a tree to the next; a program that must give the same answer on every
machine does not read them.

`size` is optional, so a sum over it states what a folder counts as:

```hql
import io.fs

folder("examples/fs/tree")
| walk
| filter(e => e.kind == "file")
| map(e => e.size | unwrap_or(0))
| sum
```

```text
90
```

A link is reported with the kind `"link"` and not followed.

## A glob

A **`Glob`** is a pattern over paths, matched against a path relative to the
folder it is applied to. `*` matches within one segment, `**` any number of
whole segments, none included, `?` one character other than `/`, `[abc]` one
character of the set and `[a-z]` one of the range. `matches` reads the text
and never the file system:

```hql
import io.fs

[
    Glob("data/*.csv").matches(Path("data/birds.csv")),
    Glob("data/*.csv").matches(Path("data/2026/q1.csv")),
    Glob("data/**/*.csv").matches(Path("data/2026/q1.csv")),
]
```

```text
[true, false, true]
```

The `glob` step gives the entries of `walk` whose path below the folder
matches, in walk order:

```hql
import io.fs

folder("examples/fs/tree") | glob(Glob("**/*.csv")) | map(e => e.path.text)
```

```text
["examples/fs/tree/data/2026/q1.csv", "examples/fs/tree/data/birds.csv"]
```

A pattern that does not parse, an unclosed `[` or a `**` that is not a whole
segment, is an evaluation error where `Glob` constructs it:

```hql
import io.fs

Glob("data/a**.csv")
```

```text
execute failure[invalid-argument]: "data/a**.csv" is not a glob: `**` stands for whole segments, alone between `/`
 --> <expression>:3:1
  |
3 | Glob("data/a**.csv")
  | ^^^^^^^^^^^^^^^^^^^^
```

## Reading what was found

An entry's `path` is one `file` reads as written, so a listing feeds
`source` ([reading with `source`](source.md)). Every row of every CSV file
directly in `data/`:

```hql
import io.fs

folder("examples/fs/tree/data")
| glob(Glob("*.csv"))
| flat_map(e => source(file(e.path), Csv))
```

```text
[{count: "2", species: "heron"}, {count: "1", species: "owl"}]
```

## A path that may not be there

Listing a folder that does not exist, or is not a folder, is an evaluation
error naming its path:

```hql
import io.fs

folder("examples/fs/nowhere") | entries
```

```text
execute failure[io-failure]: examples/fs/nowhere: no folder is there
 --> <expression>:3:33
  |
3 | folder("examples/fs/nowhere") | entries
  |                                 ^^^^^^^
```

`exists` and `entry` test a path without an error. `exists` is `false` where
nothing is there, and `entry` is `some(Entry)` or `none`:

```hql
import io.fs

[
    str(Path("examples/fs/tree/data").exists()),
    str(Path("examples/fs/tree/missing").exists()),
    Path("examples/fs/tree/data").entry() | map(e => e.kind) | unwrap_or("?"),
    str(Path("examples/fs/tree/missing.csv").entry()),
]
```

```text
["true", "false", "folder", "none"]
```

## What a host grants

`entries`, `walk`, `glob`, `exists` and `entry` are effectful, and `hql
check` reports a function that uses one:

<!-- run: check -->
```hql
import io.fs

fn csv_files() : Seq<Entry> = folder("examples/fs/tree") | glob(Glob("**/*.csv"))
csv_files() | size
```

```text
csv_files is Effectful: it uses `glob`
Int
```

A listing is granted exactly where reading a file at the same path is
([effects and what a host allows](hosts.md)): anywhere under `hql eval`,
`hql run` and the REPL, and under `hql render` only below the document's own
directory. The [reference](../../reference/std/io/fs/index.md) states each
operation of `io.fs`.
