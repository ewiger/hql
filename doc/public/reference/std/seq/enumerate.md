<!-- hql reference: std/seq.enumerate -->
# `enumerate`

In [`std/seq`](index.md).

```hql
fn enumerate(self) : List<Pair<Int, T>>
```

Each element of `self` paired with its position, counting from 0:
`["a", "b"] | enumerate` is
`[Pair(first = 0, second = "a"), Pair(first = 1, second = "b")]`.
<!-- hql reference: end -->
