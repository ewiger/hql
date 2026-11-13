<!-- hql reference: std/ai/agents.ask -->
# `ask`

In [`std/ai/agents`](index.md).

```hql
effectful fn ask(run: Run, message: String) : Run
```

Continues `run`'s conversation with `message`, and gives a new `Run`
whose `parent` is `run`. Effectful.
<!-- hql reference: end -->
