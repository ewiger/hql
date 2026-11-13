<!-- hql reference: std/ai/models.Tool -->
# `Tool`

In [`std/ai/models`](../index.md).

```hql
type Tool { name : String description : String parameters : String }
```

A function offered to a model: the `name` it calls it by, the
`description` it reads, and the JSON Schema of its `parameters`, as text.

```hql
fn Tool<F>(f: F, description: String) : Tool
```

The `Tool` that calls the declared function `f` under its own name,
with `description`. Each parameter of `f` is a `Bool`, `Int`, `Float`,
`String`, `Data`, a `List` or a `Map<String, _>` of these, or an
`Option` of one, which is not required; a lambda has no name and is
refused. Pure: `Tool(find_adrs, "Find proposals about a topic")`.
<!-- hql reference: end -->
