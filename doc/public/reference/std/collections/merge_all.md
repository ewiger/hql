<!-- hql reference: std/collections.merge_all -->
# `merge_all`

In [`std/collections`](index.md).

```hql
reduction fn merge_all<T>(self: Collection<T>) : T
```

Every map merged in order, a later key winning: `reduce({:}, merge)`.
<!-- hql reference: end -->
