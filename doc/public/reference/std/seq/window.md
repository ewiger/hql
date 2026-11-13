<!-- hql reference: std/seq.window -->
# `window`

In [`std/seq`](index.md).

```hql
fn window(self, n: Int) : List<List<T>>
```

Every run of `n` consecutive elements of `self`, in order:
`[1, 2, 3, 4] | window(2)` is `[[1, 2], [2, 3], [3, 4]]`. An `n` below 1
is an evaluation error.
<!-- hql reference: end -->
