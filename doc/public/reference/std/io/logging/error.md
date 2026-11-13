<!-- hql reference: std/io/logging.error -->
# `error`

In [`std/io/logging`](index.md).

```hql
effectful fn error(log: Logger, message: String, data: Data = {:}) : Step
```

Writes one `Error` event with `message` and `data` to `log`'s
destination, whatever `log`'s threshold, and gives `Step`. Effectful:
`log | error("unreadable", {path: "birds.json"})`.
<!-- hql reference: end -->
