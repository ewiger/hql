<!-- hql reference: std/math.int -->
# `int`

In [`std/math`](index.md).

```hql
fn int(self: Float) : Int
fn int(self: Decimal) : Int
```

The integer part of `self`, as an `Int`, the fraction dropped toward
zero: `int(-2.9f)` is `-2`. A value past the range of `Int` is an
evaluation error.
<!-- hql reference: end -->
