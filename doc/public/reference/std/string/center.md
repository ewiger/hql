<!-- hql reference: std/string.center -->
# `center`

In [`std/string`](index.md).

```hql
fn center(self, width: Int, fill: String) : String
```

`self` between `fill`s until it is `width` characters long; an odd fill
character goes after it: `"ab" | center(5, "*")` is `"*ab**"`.
<!-- hql reference: end -->
