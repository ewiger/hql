<!-- hql reference: std/core.List -->
# `List`

In [`std/core`](../index.md).

```hql
type List<T> : Seq<T>
```

A sequence of materialized occurrences; repeated traversal preserves
positions. A list literal `[1, 2, 3]` is a `List<Int>`.

```hql
fn List<T>(xs: Seq<T>) : List<T>
```

The `List` of the elements of `xs`, in order: `List([1, 2])` is
`[1, 2]`.
<!-- hql reference: end -->
