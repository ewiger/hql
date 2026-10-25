<!-- hql reference: std/io/logging.info -->
# `info`

In [`std/io/logging`](index.md).

```hql
effectful fn info(log: Logger, message: String, data: Data = {:}) : Unit
```

Writes one `Info` event with `message` and `data` to `log`'s destination
when `log`'s threshold is at most `Info`, and gives `Unit`. Effectful:
with `log = Logger(stdout, Text, name = "birds")`,
`log | info("loaded", {count: 312})` writes
`INFO birds loaded {count: 312}`.
<!-- hql reference: end -->
