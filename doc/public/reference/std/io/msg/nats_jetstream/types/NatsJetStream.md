<!-- hql reference: std/io/msg/nats_jetstream.NatsJetStream -->
# `NatsJetStream`

In [`std/io/msg/nats_jetstream`](../index.md).

```hql
type NatsJetStream : {Network, MessageTransport, Readable, Writable}
```

One subject of a JetStream stream on a NATS server. It is on the
network and carries messages, each one document, `Json` unless a format
is named. A stream is a log that does not end, so it is `Readable` and
not `Replayable`: read, it is a `Stream`.
<!-- hql reference: end -->
