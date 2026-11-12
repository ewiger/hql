<!-- hql reference: std/io/msg/nats_jetstream.nats_jetstream -->
# `nats_jetstream`

In [`std/io/msg/nats_jetstream`](index.md).

```hql
fn nats_jetstream(subject: String, stream: String, url: String = "", url_env: String = "", from: String = "earliest", idle_ms: Int = 0, consumer: String = "") : NatsJetStream
```

The subject `subject` of the stream `stream`, as a `NatsJetStream`
transport; its parameters are the keys of the `nats_jetstream` kind.
The server's address is `url`, or the value of the environment variable
`url_env` names, one of the two; an address that holds credentials is
named by `url_env`. A read starts at the `"earliest"` message the stream
keeps, or at the next one published under `from = "latest"`, and ends
once no message has arrived for `idle_ms` milliseconds, or never when
that is 0. With `consumer`, the server keeps the read's position under
that name, so a later read starts after the last message acknowledged.
Constructing it opens no connection.
<!-- hql reference: end -->
