<!-- hql reference: std/collections.product -->
# `product`

In [`std/collections`](index.md).

```hql
reduction fn product(self: Collection<Int>) : Int
reduction fn product(self: Collection<Float>) : Float
reduction fn product(self: Collection<Decimal>) : Decimal
```

The product of the occurrences, `reduce(1, *)`; `[] | product` is `1`.
<!-- hql reference: end -->
