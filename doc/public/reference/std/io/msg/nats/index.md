<!-- hql reference: std/io/msg/nats -->
# `std/io/msg/nats`

```hql
module io.msg.nats
```

NATS JetStream: one subject of a stream, as a transport read by `tap`
and written by `sink` and `target` like any other endpoint (module
`io`). The `nats` kind of a configured endpoint is this transport under
an id. A program imports the module: `import io.msg.nats`.

- [`Nats`](types/Nats.md) — One subject of a JetStream stream on a NATS server.
- [`nats`](nats.md) — The subject `subject` of the stream `stream`, as a `Nats` transport; its parameters are the keys of the `nats` kind.
<!-- hql reference: end -->
