<!-- hql reference: std/io/logging.trace -->
# `trace`

In [`std/io/logging`](index.md).

```hql
effectful fn trace(log: Logger, message: String, data: Data = {:}) : Unit
```

Writes one `Trace` event with `message` and `data` to `log`'s
destination when `log`'s threshold is `Trace`, and gives `Unit`.
Effectful. Called directly or through a pipe: `trace(log, "step")` is
`log | trace("step")`.
<!-- hql reference: end -->
