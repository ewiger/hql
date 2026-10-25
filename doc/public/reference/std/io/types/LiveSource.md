<!-- hql reference: std/io.LiveSource -->
# `LiveSource`

In [`std/io`](../index.md).

```hql
abstract type LiveSource<T> : {DataSource, Stream<T>}
```

A source of each document as it arrives, with no replay: read from a
live endpoint.
<!-- hql reference: end -->
