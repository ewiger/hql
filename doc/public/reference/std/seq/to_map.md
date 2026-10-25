<!-- hql reference: std/seq.to_map -->
# `to_map`

In [`std/seq`](index.md).

```hql
fn to_map<K, V>(self: Collection<T>, key: T -> K, value: T -> V) : Map<K, V>
```

A map from the `key` of each value of `self` to its `value`; two values
with one key are an evaluation error:
`["bb", "a"] | to_map(s => s, s => size(s))` is `Map {"bb": 2, "a": 1}`.
<!-- hql reference: end -->
