<!-- hql reference: std/io/logging.LogEvent -->
# `LogEvent`

In [`std/io/logging`](../index.md).

```hql
type LogEvent { level : LogLevel logger : String // the producer's name, dotted by convention: hql.query message : String data : Data // structured context, {:} when none is given }
```

One logged event: its `level`, the `logger` that produced it, its
`message` and its `data`. Written as `Text` it is one line, the level,
the logger and the message, then the data as an HQL literal unless it is
empty:

    INFO birds loaded {count: 312}

Written as `Json` it is one document with these four keys.
<!-- hql reference: end -->
