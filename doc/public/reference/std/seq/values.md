<!-- hql reference: std/seq.values -->
# `values`

In [`std/seq`](index.md).

```hql
fn values<K, V>(self: Map<K, V>) : List<V>
```

The values of the map `self`, in the map's order:
`Map(["b", "a"], [2, 1]) | values` is `[2, 1]`.
<!-- hql reference: end -->
