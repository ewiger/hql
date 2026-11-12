<!-- hql reference: std/time.add_years -->
# `add_years`

In [`std/time`](index.md).

```hql
fn add_years(self: Date, n: Int) : Date
fn add_years(self: DateTime, n: Int) : DateTime
```

`self` moved by `n` years, clamped as `add_months` is: February 29
moved by a year is February 28.
<!-- hql reference: end -->
