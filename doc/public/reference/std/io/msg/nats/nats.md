<!-- hql reference: std/io/msg/nats.nats -->
# `nats`

In [`std/io/msg/nats`](index.md).

```hql
fn nats(subject: String, url: String = "", url_env: String = "", idle_ms: Int = 0) : Nats
```

The subject `subject`, as a `Nats` transport; its parameters are the
keys of the `nats` kind. A read may name wildcards, `*` for one token
and `>` for every token after it; a write names one subject. The
server's address is `url`, or the value of the environment variable
`url_env` names, one of the two; an address that holds credentials is
named by `url_env`. A read ends once no message has arrived for
`idle_ms` milliseconds, or never when that is 0. Constructing it opens
no connection.
<!-- hql reference: end -->
