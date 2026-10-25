<!-- hql reference: std/collections.approx_distinct -->
# `approx_distinct`

In [`std/collections`](index.md).

```hql
reduction fn approx_distinct<T>(self: Collection<T>) : Int
```

An estimate of the number of distinct occurrences, `size(Set(xs))`;
`0` over `[]`. The estimate is deterministic: the same input gives the
same result.
<!-- hql reference: end -->
