<!-- hql reference: std/ai/agents.remember -->
# `remember`

In [`std/ai/agents`](index.md).

```hql
effectful fn remember<T>(x: T, name: String) : Card
```

Writes `x`, a run's result or a `String`, as the memory card `name`
under `[ai.memory] dir`, and gives that card. Writing the same content
again gives the same card; other content is refused. Effectful.
<!-- hql reference: end -->
