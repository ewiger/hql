<!-- hql reference: std/collections.collect -->
# `collect`

In [`std/collections`](index.md).

```hql
fn collect<T>(self: Collection<T>, n: Int) : List<List<T>>
fn collect<T>(self: Stream<T>, n: Int) : Stream<List<T>>
```

Consecutive elements in batches of `n`, in order; the last batch may be
shorter: `[1, 2, 3, 4, 5] | collect(2)` is `[[1, 2], [3, 4], [5]]`. `n`
is at least `1`. `xs | collect(n) | emit` is `xs`.
<!-- hql reference: end -->
