<!-- hql reference: std/ai/models.ToolCall -->
# `ToolCall`

In [`std/ai/models`](../index.md).

```hql
type ToolCall { id : String tool : String arguments : Data status : String output : String turn : Int }
```

One call a model requested and what it was answered: the call's `id`,
the `tool` named, its `arguments`, the `status`, `"ok"`, `"failed"` or
`"refused"`, the `output` returned to the model as JSON text, and the
index of the assistant `turn` that requested it.
<!-- hql reference: end -->
