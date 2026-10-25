<!-- hql reference: std/seq.position -->
# `position`

In [`std/seq`](index.md).

```hql
fn position(self, by: T -> Bool) : Option<Int>
```

The position of the first element of `self` that `by` accepts, counting
from 0, or `none`: `[1, 2, 3, 4] | position(x => x > 2)` is `2`.
<!-- hql reference: end -->
