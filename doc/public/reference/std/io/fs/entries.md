<!-- hql reference: std/io/fs.entries -->
# `entries`

In [`std/io/fs`](index.md).

```hql
effectful fn entries(self: Folder) : Seq<Entry>
```

The entries directly in `self`, sorted by name, an entry whose name
opens with `.` included. A folder that does not exist, or is not a
folder, is an evaluation error naming its path.
<!-- hql reference: end -->
