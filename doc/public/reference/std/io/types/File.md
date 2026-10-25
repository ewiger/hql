<!-- hql reference: std/io.File -->
# `File`

In [`std/io`](../index.md).

```hql
type File : {Replayable, Writable} { path : String end : FileEnd // file(p).end appends, and tap(file(p).end, F) follows }
```

A local file, the whole of it: read, every document it holds; written,
it is replaced. A relative `path` resolves against the directory of the
source unit that names it: the document, the program file, or the
working directory for an expression.
<!-- hql reference: end -->
