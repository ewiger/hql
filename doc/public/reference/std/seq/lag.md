<!-- hql reference: std/seq.lag -->
# `lag`

In [`std/seq`](index.md).

```hql
fn lag(self: Window<T>, n: Int = 1) : Option<T>
```

The row `n` positions before, or `none` before the first. An `n` below
0 is an evaluation error.
<!-- hql reference: end -->
