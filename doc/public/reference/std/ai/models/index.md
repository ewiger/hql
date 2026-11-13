<!-- hql reference: std/ai/models -->
# `std/ai/models`

```hql
module ai.models
```

What a model is given and what it answers: the turns of a conversation,
the tools offered to it and the calls it requests. Which model answers
is configured, never written in a program: `[ai.models.<key>]` in
`hql.toml` names a provider's wire format or an agent session host.
Imported where it is used: `import ai.models`.

- [`Turn`](types/Turn.md) — One message of a conversation with a model: its `role`, `"user"`, `"assistant"` or `"tool"`, and its `text`.
- [`Tool`](types/Tool.md) — A function offered to a model: the `name` it calls it by, the `description` it reads, and the JSON Schema of its `parameters`, as text.
- [`ToolCall`](types/ToolCall.md) — One call a model requested and what it was answered: the call's `id`, the `tool` named, its `arguments`, the `status`, `"ok"`, `"failed"` or `"refused"`, the `output` returned to the model as JSON text, and the index of the assistant `turn` that requested it.
<!-- hql reference: end -->
