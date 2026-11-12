<!-- hql reference: std/time.Zone -->
# `Zone`

In [`std/time`](../index.md).

```hql
type Zone { name : String }
```

A time zone: an IANA name the binary's database holds,
`"Europe/Berlin"`, `"UTC"`, or a fixed offset `"+02:00"`. A zone turns
a wall-clock `DateTime` into an `Instant` and back.

```hql
fn Zone(name: String) : Zone
```

The zone `name` names: an IANA name, `"UTC"`, or an offset `"±HH:MM"`.
Any other text is an evaluation error naming it.
<!-- hql reference: end -->
