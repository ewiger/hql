<!-- hql reference: std/core.Duration -->
# `Duration`

In [`std/core`](../index.md).

```hql
type Duration : {Scalar, Orderable}
```

A signed length of time in nanoseconds, ordered by length:
`Duration("PT1H30M")`. `+` and `-` add and subtract two; a day is 24
hours, and a month or a year is no `Duration`.

```hql
fn Duration(text: String) : Duration
```

The `Duration` ISO 8601 duration text reads as, in weeks, days, hours,
minutes and seconds: `Duration("PT1H30M")`, `Duration("P2DT3H")`. The
year and month designators are an evaluation error, since their length
depends on the date they start from.

```hql
fn Duration(*, days: Int = 0, hours: Int = 0, minutes: Int = 0, seconds: Int = 0, millis: Int = 0, nanos: Int = 0) : Duration
```

The `Duration` of the sum of its parts, each signed:
`Duration(hours = 1, minutes = 30)` is `PT1H30M`.
<!-- hql reference: end -->
