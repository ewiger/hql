<!-- hql reference: std/option.alternative -->
# `alternative`

In [`std/option`](index.md).

```hql
fn alternative(self, other: Option<T>) : Option<T>
```

`self` when present, and `other` otherwise:
`[1][5] | alternative([9][0])` is `9`.
<!-- hql reference: end -->
