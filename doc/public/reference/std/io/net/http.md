<!-- hql reference: std/io/net.http -->
# `http`

In [`std/io/net`](index.md).

```hql
fn http(url: String, method: String = "GET", headers: Data = {:}, timeout_ms: Int = 30000) : Http
```

The HTTP resource at `url`, as an `Http` endpoint: `method` is the
request a write sends, `"GET"` unless given, and `headers` a map of
header names to values sent with every request. A response that sends
nothing for `timeout_ms` milliseconds fails, naming the URL; the bound
is 30 seconds unless given, and a given one is above 0. Constructing it
sends nothing.
<!-- hql reference: end -->
