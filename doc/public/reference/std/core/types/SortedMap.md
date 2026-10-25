<!-- hql reference: std/core.SortedMap -->
# `SortedMap`

In [`std/core`](../index.md).

```hql
type SortedMap<K, V> : OrderedMap<K, V> where K : Orderable
```

An `OrderedMap` whose key positions follow the intrinsic total order of
`K`.

```hql
fn SortedMap<K : Orderable, V>(keys: Seq<K>, values: Seq<V>) : SortedMap<K, V>
```

The `SortedMap` from each of `keys` to the value at the same position
in `values`, its keys in the order of `K`.
<!-- hql reference: end -->
