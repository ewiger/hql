<!-- hql reference: std/ai/agents.RunEvent -->
# `RunEvent`

In [`std/ai/agents`](../index.md).

```hql
type RunEvent { kind : String run : String seq : Int data : Data }
```

Something that happened during a run, in order: its `kind`, `"prompt"`,
`"request"`, `"response"`, `"tool_call"` or `"stop"`, the `run` it
happened in, its position `seq`, and its `data`.
<!-- hql reference: end -->
