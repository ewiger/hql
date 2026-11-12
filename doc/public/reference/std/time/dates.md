<!-- hql reference: std/time.dates -->
# `dates`

In [`std/time`](index.md).

```hql
fn dates(self: Date, end: Date) : List<Date>
```

Every date from `self` up to and excluding `end`, as `range` gives the
`Int`s; empty where `end` is not after `self`.
<!-- hql reference: end -->
