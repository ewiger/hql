<!-- hql reference: std/string.size -->
# `size`

In [`std/string`](index.md).

```hql
fn size(self) : Int
```

How many characters `self` has, counting code points: `"héllo" | size`
is `5`, and `s.size() == s.chars().size()` for every `s`.
<!-- hql reference: end -->
