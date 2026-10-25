<!-- hql reference: std/collections.group -->
# `group`

In [`std/collections`](index.md).

```hql
partition fn group<T, K>(self: Collection<T>, by: T -> K) : Set<Group<K, T>>
```

The occurrences partitioned by the key `by` gives each, as a set of
`Group`s, each a key and its occurrences in input order.
<!-- hql reference: end -->
