<!-- hql reference: std/option.or_else -->
# `or_else`

In [`std/option`](index.md).

```hql
fn or_else(self, default: () -> Option<T>) : Option<T>
```

`self` when present, and the option `default` returns otherwise:
`[1][5] | or_else(() => [9][0])` is `9`.
<!-- hql reference: end -->
