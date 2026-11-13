<!-- hql reference: std/time.Zone -->
# `Zone`

In [`std/time`](../index.md).

```hql
type Zone { name : String }
```

A time zone: an IANA name the time-zone database holds,
`"Europe/Berlin"`, `"UTC"`, or a fixed offset `"+02:00"`. A zone turns
a wall-clock `DateTime` into an `Instant` and back. The database is the
host's where it has one, else the one compiled into the binary, and
`hql --version` names which: `system`, or the bundled release.

```hql
fn Zone(name: String) : Zone
```

The zone `name` names: an IANA name, `"UTC"`, or an offset `"±HH:MM"`.
Any other text is an evaluation error naming it.
<!-- hql reference: end -->
