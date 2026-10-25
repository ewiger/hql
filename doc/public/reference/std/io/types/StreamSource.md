<!-- hql reference: std/io.StreamSource -->
# `StreamSource`

In [`std/io`](../index.md).

```hql
abstract type StreamSource<T> : {DataSource, Stream<Message<T>>}
```

A log: `Message<T>`s in order, replayable within its retention.
<!-- hql reference: end -->
