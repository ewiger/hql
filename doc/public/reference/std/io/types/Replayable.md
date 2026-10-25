<!-- hql reference: std/io.Replayable -->
# `Replayable`

In [`std/io`](../index.md).

```hql
abstract type Replayable : Readable
```

A `Readable` endpoint that gives the same bytes on every read in one
evaluation, so `source` reads what it holds as a `Seq`.
<!-- hql reference: end -->
