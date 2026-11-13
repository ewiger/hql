<!-- hql reference: std/collections.variance -->
# `variance`

In [`std/collections`](index.md).

```hql
reduction fn variance(self: Collection<Int>) : Option<Float>
reduction fn variance(self: Collection<Float>) : Option<Float>
```

The population variance of the occurrences, as a `Float`, or `none` over
`[]`: `[1, 2, 3, 4] | variance` is `1.25`.
<!-- hql reference: end -->
