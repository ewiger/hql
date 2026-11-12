<!-- hql reference: std/time.in_zone -->
# `in_zone`

In [`std/time`](index.md).

```hql
fn in_zone(self: DateTime, zone: Zone) : Instant
```

The instant the wall-clock reading `self` names in `zone`. A reading in
a gap moves forward by the gap's length, and one in an overlap is the
earlier of its two instants.
<!-- hql reference: end -->
