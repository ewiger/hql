<!-- hql reference: std/collections.top_k -->
# `top_k`

In [`std/collections`](index.md).

```hql
reduction fn top_k<T : Orderable>(self: Collection<T>, k: Int) : List<T>
reduction fn top_k<T, K : Orderable>(self: Collection<T>, k: Int, by: T -> K) : List<T>
```

The `k` greatest occurrences, by value order or by the key `by` gives,
greatest first, equal keys in input order:
`[3, 1, 4, 1, 5] | top_k(2)` is `[5, 4]`.
<!-- hql reference: end -->
