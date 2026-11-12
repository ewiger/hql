<!-- hql reference: std/io/msg/nats -->
# `std/io/msg/nats`

```hql
module io.msg.nats
```

Core NATS: one subject, as a transport read by `tap` and written by
`sink` and `target` like any other endpoint (module `io`). The server
keeps nothing: a read is given what is published while it is
subscribed, and a message published while no one is subscribed reaches
no one. A stream that keeps a subject's messages is
`io.msg.nats_jetstream`. The `nats` kind of a configured endpoint is
this transport under an id. A program imports the module:
`import io.msg.nats`.

- [`Nats`](types/Nats.md) — One core NATS subject on a server.
- [`nats`](nats.md) — The subject `subject`, as a `Nats` transport; its parameters are the keys of the `nats` kind.
<!-- hql reference: end -->
