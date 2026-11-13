<!-- hql reference: std/seq.slice -->
# `slice`

In [`std/seq`](index.md).

```hql
fn slice(self, range: Range) : Seq<T>
```

The elements of `self` within `range`, clamped to the sequence, a
negative bound counting from the end: `[1, 2, 3][1:]` is `[2, 3]`.
<!-- hql reference: end -->
