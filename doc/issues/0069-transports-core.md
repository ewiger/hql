# 0069 — Transports: the core of HQL-0047 and HQL-0048

Status: todo
Related: [HQL-0047](../proposals/HQL-0047/README.md) (transports),
[HQL-0048](../proposals/HQL-0048/README.md) (I/O transport namespaces),
[HQL-0012](../proposals/HQL-0012/README.md) (data sources, amended by both),
[HQL-0049](../proposals/HQL-0049/README.md) (NATS and lakehouse tables, built
on both), [`lib/std/io/mod.hql`](../../lib/std/io/mod.hql),
[`lib/std/io/net/mod.hql`](../../lib/std/io/net/mod.hql),
[`src/endpoints.rs`](../../src/endpoints.rs), [`src/net.rs`](../../src/net.rs),
[endpoints](../public/learn/15-endpoints.md)

HQL-0047 and HQL-0048 are drafted, yet part of them has shipped and accepted
records stand on them. HQL-0012 names both as amendments, and HQL-0049
declares its NATS modules by their rules. What exists:

- `Transport`, `Direct`, `Network` and the three carriages are declared in
  `io`.
- `io.db.sqlite`, `io.msg.nats` and `io.msg.nats_jetstream` are modules with
  a transport type and a constructor.

What does not:

- `Stdin`, `Stdout`, `Stderr`, `File`, `FileEnd`, `Http`, `WebSocket` and
  `Socket` have no locality and no carriage among their supertypes.
- `hql render` refuses the network per constructor, in
  `src/extensions/net.rs` and again in `src/extensions/nats.rs`, and not by
  one check over `Network`. Every new transport copies the refusal.
- `postgres`, `spark` and `kafka` are kinds only: no type, no constructor, no
  module, no reference page. The endpoints chapter says
  `import io.db.postgres` brings `postgres(...)` into scope, and it does not.
- `socket` is not renamed `tcp`; the endpoints chapter lists `tcp`.
- `http`, `websocket` and `tcp` are not kinds, so a request that needs
  credentials puts its header in program text.
- An `http` read has no timeout: a server that accepts and never answers
  holds the evaluation forever.

## Scope

In the order of HQL-0047's deployment, each step committed with its tests:

1. `socket` to `tcp`, `Socket` to `Tcp`, with a breaking-change entry in
   `CHANGELOG.md`.
2. The locality and carriage of every existing type, and one host check that
   reads `Network` from the type and replaces the per-constructor refusals.
3. `io.db.postgres`, `io.db.spark` and `io.msg.kafka`: types and
   constructors resolving to the readers and writers their kinds have, and
   reference pages.
4. `http`, `websocket` and `tcp` as configured kinds, with `format`,
   `headers` and `token_env`.
5. Network bounds: a connection fails after 10 seconds and an `http`
   response silent for 30 seconds fails, each naming the URL or address. A
   `timeout_ms` parameter and key overrides the response bound, as `idle_ms`
   bounds a NATS or Kafka read; HQL-0047 is amended to state it.

Out of scope: `rest`, `graphql` and `webhook`, and every module HQL-0048
marks as design direction. Each is its own issue.

## Done when

- Every concrete transport has one locality and one carriage, checked by
  `hql check`.
- A network transport, constructed or configured, is refused under
  `hql render` by the one check, with the one message.
- `import io.db.postgres`, `import io.db.spark` and `import io.msg.kafka`
  check, and a constructed transport and a configured id over one table or
  topic give the same rows or messages.
- A configured `http` id with `token_env` sends `Authorization`.
- A loopback server that never answers ends in a failure naming its URL.
- The endpoints chapter runs as written.
- HQL-0047 and HQL-0048 read "accepted, implemented in part", with
  `rest`, `graphql`, `webhook` and the design-direction modules as the part
  left.
