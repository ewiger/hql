<!-- hql reference: std/seq.reverse -->
# `reverse`

In [`std/seq`](index.md).

```hql
fn reverse<F<_> : Seq<_>>(self: F<T>) : F<T>
```

The elements of `self` in the opposite order, in its own constructor:
`[1, 2, 3] | reverse` is `[3, 2, 1]`. A `Ranking` reversed is a
`Seq<Hit<T>>`: the new order is not the retrieval's.
<!-- hql reference: end -->
