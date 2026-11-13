<!-- hql reference: std/ai/agents.run -->
# `run`

In [`std/ai/agents`](index.md).

```hql
effectful fn run<T>(input: T, agent: Agent, *, tools: List<Tool> = []) : Run
```

Executes `agent` over `input`, a `Context` or a `Prompt`, with `tools`,
which must be exactly the tools the agent's card lists, and gives the
`Run`. Effectful: it reaches a model only as `--ai` allows.
<!-- hql reference: end -->
