<!-- hql reference: std/seq.range -->
# `range`

In [`std/seq`](index.md).

```hql
fn range(start: Int, end: Int, step: Int = 1) : List<Int>
```

The `Int`s from `start` up to, not including, `end`, each `step` after
the one before: `range(1, 6)` is `[1, 2, 3, 4, 5]`, and
`range(10, 0, -3)` is `[10, 7, 4, 1]`.
<!-- hql reference: end -->
