<!-- hql reference: std/seq.min_by -->
# `min_by`

In [`std/seq`](index.md).

```hql
fn min_by<K : Orderable>(self: Collection<T>, by: T -> K) : Option<T>
fn min_by(self: Collection<T>, by: (T, T) -> Ordering) : Option<T>
```

The value of `self` that is least, by the key `by` gives or by the
`Ordering` `by` gives two values; the first of equals, or `none` over an
empty collection: `["bb", "a", "ccc"] | min_by(s => size(s))` is `"a"`.
<!-- hql reference: end -->
