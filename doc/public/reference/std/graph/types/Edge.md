<!-- hql reference: std/graph.Edge -->
# `Edge`

In [`std/graph`](../index.md).

```hql
type Edge<S, T> { source : String target : String data : Data }
```

An edge from `source` to `target`, with `data` about it. Whatever `S`
and `T` are, each end is held as the name of the node it is.
<!-- hql reference: end -->
