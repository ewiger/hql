<!-- hql reference: std/io/fs.Entry -->
# `Entry`

In [`std/io/fs`](../index.md).

```hql
type Entry { path : Path name : String kind : String size? : Int }
```

One thing a folder holds: its `path`, the listed folder's path joined
with the entry's path below it, so `file(e.path)` reads it; its `name`,
the last segment; its `kind`, `"file"`, `"folder"` or `"link"`; and its
`size`, a file's length in bytes, absent for a folder or a link. A link
is reported and not followed.
<!-- hql reference: end -->
