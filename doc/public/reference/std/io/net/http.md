<!-- hql reference: std/io/net.http -->
# `http`

In [`std/io/net`](index.md).

```hql
fn http(url: String, method: String = "GET", headers: Data = {:}) : Http
```

The HTTP resource at `url`, as an `Http` endpoint: `method` is the
request a write sends, `"GET"` unless given, and `headers` a map of
header names to values sent with every request. Constructing it sends
nothing.
<!-- hql reference: end -->
