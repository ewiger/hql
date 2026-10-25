<!-- hql reference: std/io/logging.Cell -->
# `Cell`

In [`std/io/logging`](../index.md).

```hql
type Cell { logs : Channel<LogEvent> }
```

The cell `hql render` is running, bound as `cell`. The events written
to its `logs` are shown beside the cell's result and never saved into
the document.
<!-- hql reference: end -->
