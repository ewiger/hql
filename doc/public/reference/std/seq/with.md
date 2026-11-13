<!-- hql reference: std/seq.with -->
# `with`

In [`std/seq`](index.md).

```hql
fn with<M : Map<K, V>, K, V>(self: M, key: K, value: V) : M
```

The map `self` with `key` set to `value`, added or replaced, at the type
of `self`: `Map(["a"], [1]) | with("b", 2)` is `Map {"a": 1, "b": 2}`.
<!-- hql reference: end -->
