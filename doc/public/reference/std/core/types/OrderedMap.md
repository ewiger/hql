<!-- hql reference: std/core.OrderedMap -->
# `OrderedMap`

In [`std/core`](../index.md).

```hql
type OrderedMap<K, V> : Map<K, V>
```

A `Map` whose keys keep their construction positions: its keys are a
`Seq<K>`. `K` need not be `Orderable`. Viewed as a `Map`, its keys are
a set.

```hql
fn OrderedMap<K, V>(keys: Seq<K>, values: Seq<V>) : OrderedMap<K, V>
```

The `OrderedMap` from each of `keys` to the value at the same position
in `values`, its keys in the order of `keys`.
<!-- hql reference: end -->
