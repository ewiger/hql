<!-- hql reference: std/core.Instant -->
# `Instant`

In [`std/core`](../index.md).

```hql
type Instant : {Scalar, Orderable}
```

A point on the UTC timeline with nanosecond precision, ordered by time:
`Instant("2026-10-24T07:30:00Z")`. It is written in RFC 3339, in UTC.

```hql
fn Instant(text: String) : Instant
```

The `Instant` RFC 3339 text reads as, with `Z` or an offset:
`Instant("2026-10-24T09:30:00+02:00")` is `2026-10-24T07:30:00Z`. Text
without an offset is a `DateTime`'s, and an evaluation error here.

```hql
fn Instant(*, unix_seconds: Int) : Instant
```

The `Instant` `unix_seconds` whole seconds after `1970-01-01T00:00:00Z`,
before it when negative.

```hql
fn Instant(*, unix_millis: Int) : Instant
```

The `Instant` `unix_millis` milliseconds after `1970-01-01T00:00:00Z`.
<!-- hql reference: end -->
