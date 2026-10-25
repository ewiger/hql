<!-- hql reference: std/string.to_decimal -->
# `to_decimal`

In [`std/string`](index.md).

```hql
fn to_decimal(self) : Option<Decimal>
```

The exact decimal number `self` writes, at its written scale, or `none`
when it writes none: `"2.50" | to_decimal` is `2.50`.
<!-- hql reference: end -->
