<!-- hql reference: std/ai/agents.delegate -->
# `delegate`

In [`std/ai/agents`](index.md).

```hql
effectful fn delegate(run: Run, agent: Agent, task: String, *, tools: List<Tool> = []) : Run
```

Hands `run`'s context and answer to `agent` with `task`, and gives a
new `Run` whose `parent` is `run`. Effectful.
<!-- hql reference: end -->
