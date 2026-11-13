<!-- hql reference: std/seq.max_by -->
# `max_by`

In [`std/seq`](index.md).

```hql
fn max_by<K : Orderable>(self: Collection<T>, by: T -> K) : Option<T>
fn max_by(self: Collection<T>, by: (T, T) -> Ordering) : Option<T>
```

The value of `self` that is greatest, by the key `by` gives or by the
`Ordering` `by` gives two values; the first of equals, or `none` over an
empty collection: `["bb", "a", "ccc"] | max_by(s => size(s))` is `"ccc"`.
<!-- hql reference: end -->
