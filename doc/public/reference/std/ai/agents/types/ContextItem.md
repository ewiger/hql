<!-- hql reference: std/ai/agents.ContextItem -->
# `ContextItem`

In [`std/ai/agents`](../index.md).

```hql
type ContextItem { label : String kind : String text : String score? : Float provenance : Data }
```

One part of a context: the `label` of the part it belongs to, the
`kind`, the type of the value it was made from, its `text` as sent, its
`score` when the value was a hit, and its `provenance`: a card's name
and path, a hit's retrieval, or a run's id.
<!-- hql reference: end -->
