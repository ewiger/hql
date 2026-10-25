<!-- hql reference: std/string.join -->
# `join`

In [`std/string`](index.md).

```hql
fn join(self: Collection<String>, separator: String) : String
```

The texts of `self` in order, with `separator` between each two:
`["a", "b"].join(", ")` is `"a, b"`. The one function here whose
receiver is not a `String`, so the receiver is written.
<!-- hql reference: end -->
