<!-- hql reference: std/time.parse_instant -->
# `parse_instant`

In [`std/time`](index.md).

```hql
fn parse_instant(self: String, pattern: String) : Option<Instant>
```

`self` read through a strftime `pattern` as an `Instant`, or `none`; the
pattern writes an offset with `%z` or a zone with `%Z`.
<!-- hql reference: end -->
