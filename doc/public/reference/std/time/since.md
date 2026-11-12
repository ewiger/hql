<!-- hql reference: std/time.since -->
# `since`

In [`std/time`](index.md).

```hql
fn since(self: Instant, earlier: Instant) : Duration
fn since(self: DateTime, earlier: DateTime) : Duration
```

The duration from `earlier` to `self`, negative where `earlier` is after
`self`. Over `DateTime` it is wall-clock time, independent of any zone.
<!-- hql reference: end -->
