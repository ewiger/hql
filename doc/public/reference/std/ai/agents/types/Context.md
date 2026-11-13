<!-- hql reference: std/ai/agents.Context -->
# `Context`

In [`std/ai/agents`](../index.md).

```hql
type Context : Seq<ContextItem> { items : List<ContextItem> }
```

What a run is given to read: a sequence of items, combined with
`concat` and narrowed with `filter` as any sequence is.

```hql
fn Context<T>(x: T, label: String = "context") : Context
```

The `Context` of `x` under `label`: one item for each element of a
sequence, in its order, and one for any other value. An item's text is
what `markdown` writes for its value; a hit keeps its score and
retrieval, a card its name and path. Pure.
<!-- hql reference: end -->
