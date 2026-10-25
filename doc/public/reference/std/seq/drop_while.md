<!-- hql reference: std/seq.drop_while -->
# `drop_while`

In [`std/seq`](index.md).

```hql
fn drop_while<S : Seq<T>>(self: S, by: T -> Bool) : S
```

The values of `self` after the leading ones `by` accepts, at the type of
`self`: `[1, 2, 5, 1] | drop_while(x => x < 3)` is `[5, 1]`.
<!-- hql reference: end -->
