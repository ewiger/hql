<!-- hql reference: std/collections.count_by -->
# `count_by`

In [`std/collections`](index.md).

```hql
reduction fn count_by<T, K>(self: Collection<T>, by: T -> K) : Map<K, Int>
```

The number of occurrences with each key `by` gives, as a map from key
to count; empty over `[]`.
`["a", "bb", "c"] | count_by(s => size(s))` is `Map {1: 2, 2: 1}`.
<!-- hql reference: end -->
