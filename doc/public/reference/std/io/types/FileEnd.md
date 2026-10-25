<!-- hql reference: std/io.FileEnd -->
# `FileEnd`

In [`std/io`](../index.md).

```hql
type FileEnd : {Readable, Writable} { path : String }
```

The end of a file, after what it holds, as `file(path).end`: a write
there appends, and `tap` follows what is written next, as `tail -f`
does. It is live, not `Replayable`, so `source` refuses it.
<!-- hql reference: end -->
