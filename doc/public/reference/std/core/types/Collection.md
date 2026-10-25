<!-- hql reference: std/core.Collection -->
# `Collection`

In [`std/core`](../index.md).

```hql
abstract type Collection<T> : Iterable<T>
```

A finite multiset of occurrences, unordered, with no runtime
representation. `size(c)` is the number of occurrences, `count(c, x)`
the number equal to `x`, and `contains(c, x)` holds where
`count(c, x) > 0`; so `size(c)` is the sum of `count(c, x)` over the
distinct values in `c`.
<!-- hql reference: end -->
