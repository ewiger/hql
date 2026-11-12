<!-- hql reference: std/time.iso_year -->
# `iso_year`

In [`std/time`](index.md).

```hql
fn iso_year(self: Date) : Int
fn iso_year(self: DateTime) : Int
```

The year the ISO week of `self` belongs to: `2027` for `2026-12-31`
where that day falls in the first week of 2027.
<!-- hql reference: end -->
