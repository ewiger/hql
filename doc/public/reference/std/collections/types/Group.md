<!-- hql reference: std/collections.Group -->
# `Group`

In [`std/collections`](../index.md).

```hql
type Group<K, T> { key : K items : List<T> }
```

One partition of a `group`: its `key`, and the occurrences with that key
in input order, as `items`. Two groups are equal iff their keys are.
<!-- hql reference: end -->
