<!-- hql reference: std/seq.without -->
# `without`

In [`std/seq`](index.md).

```hql
fn without<M : Map<K, V>, K, V>(self: M, key: K) : M
```

The map `self` without `key`, whether or not it held it, at the type of
`self`: `Map(["a", "b"], [1, 2]) | without("a")` is `Map {"b": 2}`.
<!-- hql reference: end -->
