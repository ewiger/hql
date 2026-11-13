<!-- hql reference: std/io/fs -->
# `std/io/fs`

```hql
module io.fs
```

The file system as data: folders, globs over paths, and the entries a
folder holds, with their kinds, sizes and modification times.
Constructing a `Folder` or a `Glob` opens, stats and resolves nothing;
the listing operations read the file system, each once, where it is
written, and give a `Seq<Entry>` in the order of a sorted walk, never
the file system's. A listing is granted exactly where reading a file at
the same path is. A program imports the module: `import io.fs`, and
reaches it as `fs`.

- [`Folder`](types/Folder.md) — A folder of the file system at `path`: an endpoint that holds files and folders, as a database holds tables.
- [`Glob`](types/Glob.md) — A pattern over paths, matched against a path relative to the folder it is applied to: `*` matches within one segment, `**` any number of whole segments, `?` one character other than `/`, and `[abc]` one character of the set, `[a-z]` one of the range.
- [`Entry`](types/Entry.md) — One thing a folder holds: its `path`, the listed folder's path joined with the entry's path below it, so `file(e.path)` reads it; its `name`, the last segment; its `kind`, `"file"`, `"folder"` or `"link"`; its `size`, a file's length in bytes, absent for a folder or a link; and its `modified`, the time the entry was last modified, a link's own and not its target's, absent where the platform does not record one.
- [`folder`](folder.md) — The `Folder` at `path`, a `String` or a `Path`.
- [`matches`](matches.md) — Whether `path`, read as relative to the folder the glob is applied to, matches `self`.
- [`entries`](entries.md) — The entries directly in `self`, sorted by name, an entry whose name opens with `.` included.
- [`walk`](walk.md) — Every entry below `self`, the folder itself excluded: depth first, each folder's entries sorted by name, and each folder followed at once by what it holds.
- [`glob`](glob.md) — The entries of `walk(self)` whose path below `self` matches `pattern`, in walk order.
- [`exists`](exists.md) — Whether something is at `self`: a file, a folder or a link.
- [`entry`](entry.md) — The `Entry` at `self`, `some(Entry)`, or `none` where nothing is there.
<!-- hql reference: end -->
