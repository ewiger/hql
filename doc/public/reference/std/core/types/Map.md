<!-- hql reference: std/core.Map -->
# `Map`

In [`std/core`](../index.md).

```hql
type Map<K, V>
```

Materialized associations from distinct keys of `K` to values of `V`;
not a `Collection`. Its keys are a `Set<K>`, and a lookup gives an
`Option<V>`.

```hql
fn Map<K, V>(keys: Seq<K>, values: Seq<V>) : Map<K, V>
```

The `Map` from each of `keys` to the value at the same position in
`values`. The two are equally long, and a repeated key fails at run
time.
<!-- hql reference: end -->
