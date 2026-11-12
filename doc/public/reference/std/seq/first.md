<!-- hql reference: std/seq.first -->
# `first`

In [`std/seq`](index.md).

```hql
fn first(self: Window<T>) : T
fn first(self) : Option<T>
```

The element at position 0, or `none` over an empty sequence:
`[3, 1, 2] | first` is `3`. Of a window, the first row of its partition.
<!-- hql reference: end -->
