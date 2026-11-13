<!-- hql reference: std/collections.flat_map -->
# `flat_map`

In [`std/collections`](index.md).

```hql
elementwise fn flat_map<T, U>(self: Collection<T>, by: T -> Seq<U>) : List<U>
elementwise fn flat_map<T, U>(self: Stream<T>, by: T -> Seq<U>) : Stream<U>
```

Each element through `by` to a sequence, and the members of those
sequences in order: `map(by) | emit`.
`[1, 2] | flat_map(x => [x, x])` is `[1, 1, 2, 2]`.
<!-- hql reference: end -->
