<!-- hql reference: std/collections.concat_all -->
# `concat_all`

In [`std/collections`](index.md).

```hql
reduction fn concat_all<U>(self: Collection<Seq<U>>) : List<U>
```

Every member of every sequence, in order: `reduce([], concat)`.
`[[1], [2, 3]] | concat_all` is `[1, 2, 3]`.
<!-- hql reference: end -->
