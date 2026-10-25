<!-- hql reference: std/retrieval.Hit -->
# `Hit`

In [`std/retrieval`](../index.md).

```hql
type Hit<T> { value : T score : Float query : String retrieval : Data }
```

One ranked value: the `value`, its `score` against `query`, and the
`retrieval` that produced it, a tree holding `index`, `model`,
`revision`, `metric`, `approximate` and `query`.
<!-- hql reference: end -->
