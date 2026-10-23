# HQL-0040: `io.fs`: paths, folders, globs and entries

**Status**: drafted
**Created**: 2026-10-23
**Source**: [HQL-0021](../HQL-0021/README.md), whose non-goals exclude
directory listing and file metadata, and the sorted walk of
[HQL-0020](../HQL-0020/README.md)

## Abstract

A standard module `io.fs` makes the file system queryable. It declares
`Folder`, an endpoint that holds files and folders; `Glob`, a pattern over
paths; and `Entry`, one thing a folder holds with its kind and size. Its
operations list one folder, walk a tree, match a glob and test a path, each
effectful and each yielding a `Seq<Entry>` in the order of a sorted walk,
never the file system's. `Path`, a place in a file system written as text, is
declared in `io`, so that `file` and `io.fs` share it. The `io` extension
implements both modules. A program imports `io.fs`; the prelude does not.

## Motivation

- A program can read a file whose path it already knows, `file(p)`, and
  nothing else: it cannot ask what a folder holds, whether a path exists, or
  how large a file is.
- The vault already walks a folder in sorted order, but only to read documents
  as cards. The walk is not available for any other file, and a program that
  wants the images, the CSV files or the folders of a vault has no way to
  name them.
- Without listing, every path a program reads is written into it, so a query
  over "every CSV under `data/`" is impossible to write.

## Non-goals

- Deleting, renaming, moving or copying files, and creating folders. These
  change the file system rather than read it.
- File times. `Date` is a calendar date with no time of day; a modification
  time waits for a timestamp type.
- Owners, permissions and other platform-specific metadata.
- Following symbolic links.
- Watching a folder for changes, `tap(folder(p))`.

## Specification

### 1. Ownership

The declarations divide over two modules, by who needs them.

- `Path` MUST be declared in `io`, beside `File`. `io`'s `file` and `io.fs`
  both need it, and a parent never imports its child
  ([HQL-0028](../HQL-0028/README.md)), so the shared type sits in the parent.
- `Folder`, `Glob`, `Entry` and the listing operations MUST be declared in
  `io.fs`, at `std/io/fs/mod.hql`, which imports `core` and `io`.
- The `io` extension's descriptor MUST provide both `io` and `io.fs`. One
  extension provides any number of modules
  ([HQL-0024, modules](../HQL-0024/parts/01-modules.md)).
- `io.fs` MUST NOT be in the prelude. A program writes `import io.fs` and
  reaches it as `fs`, as it writes `import io.net` today.

### 2. `Path`

A path is text naming a place in a file system, and constructing one touches
nothing.

```hql
// std/io/mod.hql
type Path : Orderable {
    text : String
}

fn Path(text: String) : Path
fn file(path: Path) : File
fn join(self: Path, segment: String) : Path
fn parent(self: Path) : Option<Path>
fn name(self: Path) : String
fn stem(self: Path) : String
fn extension(self: Path) : Option<String>
```

- A relative path MUST resolve against the directory of the source unit that
  names it, as a `file` path does ([HQL-0021](../HQL-0021/README.md)).
- Paths MUST be ordered by comparing their segments in turn, each by its
  text; this is the order a sorted walk visits them in.
- `join`, `parent`, `name`, `stem` and `extension` MUST be pure: they read the
  text and never the file system. `Path("a/b.csv").extension()` is
  `some("csv")`, and `Path("a").parent()` is `none`.
- `file(path: String)` MUST stay, and `file(path: Path)` MUST mean
  `file(path.text)`.

### 3. `Folder`, `Glob` and `Entry`

`io.fs` declares three types, and constructing each opens, stats and resolves
nothing.

```hql
// std/io/fs/mod.hql
import core
import io

type Folder : Catalog<File> {
    path : Path
}

type Glob {
    pattern : String
}

type Entry {
    path  : Path     // the folder's path joined with the entry's own
    name  : String   // the last segment
    kind  : String   // "file", "folder" or "link"
    size? : Int      // a file's length in bytes
}

fn folder(path: String) : Folder
fn folder(path: Path) : Folder
fn Glob(pattern: String) : Glob
fn matches(self: Glob, path: Path) : Bool
```

- A folder MUST be a `Catalog<File>`: an endpoint that contains sources, as a
  database contains tables. It is not a `Seq<File>` itself.
- An entry's `path` MUST be one `file` can read as written: the listed
  folder's path joined with the entry's path below it.
- `size` MUST be present for a file and absent for a folder or a link.
- A link MUST be reported as `kind = "link"` and MUST NOT be followed.
- A glob MUST match against the path relative to the folder it is applied
  to: `*` matches within one segment, `**` any number of whole segments, `?`
  one character other than `/`, and `[abc]` one character of the set. A
  pattern that does not parse MUST be an evaluation error at `Glob(…)`.
- `matches` MUST be pure.

### 4. Listing

Listing reads the file system, so each listing operation is effectful and
runs once, where it is written.

```hql
// std/io/fs/mod.hql
effectful fn entries(self: Folder) : Seq<Entry>
effectful fn walk(self: Folder) : Seq<Entry>
effectful fn glob(self: Folder, pattern: Glob) : Seq<Entry>
effectful fn exists(self: Path) : Bool
effectful fn entry(self: Path) : Option<Entry>
```

- `entries` MUST give the entries directly in the folder, sorted by name.
- `walk` MUST give every entry below the folder, the folder itself excluded,
  depth first, each folder's entries sorted by name and each folder followed
  at once by what it holds. This MUST be the order of the sorted walk
  [HQL-0020](../HQL-0020/README.md) states for `vault.cards`, so the cards of
  a vault are the documents of `walk(folder(root))`, in the same order.
- `glob(f, g)` MUST equal `walk(f) | filter(e => g.matches(relative))`,
  where `relative` is the entry's path below `f`.
- A listing MUST be read once per evaluation and held, so it is a `Seq`: two
  traversals of one listing give the same entries, whatever changes on disk
  between them.
- `exists` and `entry` MUST answer for a missing path, `false` and `none`,
  without an error. Listing a folder that does not exist, or that is not a
  folder, MUST be an evaluation error naming the path.

### 5. Host grants

A listing reveals names, so it is held to the grants that reading is.

- Listing, walking, globbing, `exists` and `entry` MUST be allowed exactly
  where reading a file at the same path is allowed
  ([HQL-0021](../HQL-0021/README.md)). Under `hql render` that is the
  document's directory and below.
- A walk MUST NOT leave the granted directory, whether by `..` or by a link;
  since links are not followed, a walk never reaches through one.
- The checker MUST NOT stat, open or list anything: a `Folder` or a `Path` is
  a value until a listing operation runs.

### 6. Examples

```hql
import io.fs

// The CSV files under data/, largest first.
folder("data")
| glob(Glob("**/*.csv"))
| sort(by = e => e.size | unwrap_or(0))
| reverse

// The folders directly in the current one.
folder(".") | entries | filter(e => e.kind == "folder") | map(e => e.name)

// How many bytes the files under assets/ take.
folder("assets")
| walk
| filter(e => e.kind == "file")
| map(e => e.size | unwrap_or(0))
| sum

// Every row of every CSV file under data/, read through `file`.
folder("data")
| glob(Glob("*.csv"))
| flat_map(e => source(file(e.path), Csv))
```

## Backwards Compatibility

- Additive. `file(path: String)` keeps its meaning, and no existing name
  changes.
- This narrows a non-goal of [HQL-0021](../HQL-0021/README.md): directory
  listing and file sizes move in, while deletion, renaming and file times
  stay out.
- `fs` becomes a local name in a unit that imports `io.fs`; a program binding
  `fs` itself imports it under another name, `import io.fs as files`.

## Security Considerations

- A listing discloses the names and sizes of files, which is why it needs the
  same grant as reading them (section 5).
- A walk over a large tree is bounded by the run-context limits of
  [HQL-0012](../HQL-0012/README.md), as any held value is; exceeding one is an
  evaluation error, never a truncated listing.
- Not following links rules out cycles and escapes through a link.

## Reference Implementation

- `std/io/mod.hql`: `Path`, its constructor, its pure operations and the
  `file(path: Path)` form.
- `std/io/fs/mod.hql`: the module, and its entry in `library::MODULES` after
  `io`.
- `src/extensions/io.rs`: the `io` extension's descriptor provides `io.fs`;
  the listing operations, sharing the vault's sorted walk.
- The vault's walk and `walk` share one implementation, so section 4's order
  rule holds by construction.

## Test Plan

- Order: `walk` over a fixture tree equals the hand-written sorted order, and
  equals the order of `vault.cards` over the same tree's documents.
- Globs: each pattern form, a pattern that does not parse, and `glob` equal to
  its longhand.
- Entries: kinds, sizes present and absent, a link reported and not followed.
- Grants: under `hql render`, a listing outside the document's directory is
  refused, by `..` and by a link.
- `exists` and `entry` over a missing path; listing a missing folder refused
  with the path named.
- Stability: two traversals of one listing are equal after a file is added
  between them.

## Changelog

- 2026-10-23: drafted
