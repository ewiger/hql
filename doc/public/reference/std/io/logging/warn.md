<!-- hql reference: std/io/logging.warn -->
# `warn`

In [`std/io/logging`](index.md).

```hql
effectful fn warn(log: Logger, message: String, data: Data = {:}) : Unit
```

Writes one `Warn` event with `message` and `data` to `log`'s destination
when `log`'s threshold is at most `Warn`, and gives `Unit`. Effectful:
`log | warn("no sightings")`.
<!-- hql reference: end -->
