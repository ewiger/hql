<!-- hql reference: std/io/logging.debug -->
# `debug`

In [`std/io/logging`](index.md).

```hql
effectful fn debug(log: Logger, message: String, data: Data = {:}) : Unit
```

Writes one `Debug` event with `message` and `data` to `log`'s
destination when `log`'s threshold is at most `Debug`, and gives `Unit`.
Effectful: `log | debug("parsed", {rows: 3})`.
<!-- hql reference: end -->
