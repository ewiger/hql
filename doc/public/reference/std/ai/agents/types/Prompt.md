<!-- hql reference: std/ai/agents.Prompt -->
# `Prompt`

In [`std/ai/agents`](../index.md).

```hql
type Prompt { instructions : String context : Context rendered : String }
```

The model input: the authored `instructions`, the `context`, and the
`rendered` user turn as sent, the instructions and then each label's
items under a heading of its name.

```hql
fn Prompt(context: Context, instructions: String = "") : Prompt
```

The `Prompt` of `context` with `instructions`. Inside a cell the
instructions default to the cell's section: the prose from the nearest
heading above the cell to the cell. Pure.
<!-- hql reference: end -->
