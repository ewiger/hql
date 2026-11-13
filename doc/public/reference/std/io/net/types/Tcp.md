<!-- hql reference: std/io/net.Tcp -->
# `Tcp`

In [`std/io/net`](../index.md).

```hql
type Tcp : {Readable, Writable} { host : String port : Int }
```

A TCP connection to `host` on `port`. A TCP stream carries bytes, not
messages, so the documents on it are separated as their format
separates them: whitespace between `Json` values, a line for `Lines`,
`JsonLines` and `Csv`, a `---` line for `Yaml`. A connection not made
within 10 seconds fails, naming the address; a read waits for the next
document without limit.
<!-- hql reference: end -->
