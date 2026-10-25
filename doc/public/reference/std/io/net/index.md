<!-- hql reference: std/io/net -->
# `std/io/net`

```hql
module io.net
```

Network endpoints: transports, read and written by `source`, `tap`,
`target` and `sink` like any other endpoint (module `io`). Constructing
one sends nothing: the request a program describes happens when a
source, tap, target or sink uses it, and only if the host grants the
network to that program. A document reaches no network. A program
imports the module: `import net`.

- [`Http`](types/Http.md) — An HTTP resource at `url`.
- [`WebSocket`](types/WebSocket.md) — A WebSocket connection to `url`: each message is a document, read as it arrives and written as it is sent.
- [`Socket`](types/Socket.md) — A TCP connection to `host` on `port`.
- [`http`](http.md) — The HTTP resource at `url`, as an `Http` endpoint: `method` is the request a write sends, `"GET"` unless given, and `headers` a map of header names to values sent with every request.
- [`websocket`](websocket.md) — The WebSocket at `url`, as a `WebSocket` endpoint.
- [`socket`](socket.md) — The TCP endpoint `host` on `port`, as a `Socket`.
<!-- hql reference: end -->
