<!-- hql reference: std/time.fmt -->
# `fmt`

In [`std/time`](index.md).

```hql
fn fmt(self: Date, pattern: String) : String
fn fmt(self: DateTime, pattern: String) : String
```

`self` written through a strftime `pattern`, the directives `parse_date`
reads: `Date("2026-10-24").fmt("%d %B %Y")` is `"24 October 2026"`.

```hql
fn fmt(self: Instant, pattern: String) : String
fn fmt(self: Instant, pattern: String, zone: Zone) : String
```

`self` written through a strftime `pattern` as its reading in UTC, or in
`zone` where one is given.
<!-- hql reference: end -->
