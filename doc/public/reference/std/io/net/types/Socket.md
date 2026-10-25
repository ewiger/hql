<!-- hql reference: std/io/net.Socket -->
# `Socket`

In [`std/io/net`](../index.md).

```hql
type Socket : {Readable, Writable} { host : String port : Int }
```

A TCP connection to `host` on `port`. A socket carries bytes, not
messages, so the documents on it are separated as their format
separates them: whitespace between `Json` values, a line for `Lines`,
`JsonLines` and `Csv`, a `---` line for `Yaml`.
<!-- hql reference: end -->
