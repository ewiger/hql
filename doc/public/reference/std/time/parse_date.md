<!-- hql reference: std/time.parse_date -->
# `parse_date`

In [`std/time`](index.md).

```hql
fn parse_date(self: String, pattern: String) : Option<Date>
```

`self` read through a strftime `pattern` as a `Date`, or `none` where it
does not match: `"24.10.2026".parse_date("%d.%m.%Y")`. The directives
are `%Y %m %d %H %M %S %f %z %Z %j %a %A %b %B %p %%`; any other is an
evaluation error.

```hql
fn parse_date(self: String) : Option<Date>
```

`self` read as an ISO 8601 date, or `none`: with no pattern,
`parse_date` is `to_date`.
<!-- hql reference: end -->
