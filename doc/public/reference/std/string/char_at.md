<!-- hql reference: std/string.char_at -->
# `char_at`

In [`std/string`](index.md).

```hql
fn char_at(self, position: Int) : Option<String>
```

The character of `self` at `position`, counting from 0, or `none`
outside the text: `"hello" | char_at(1)` is `"e"`.
<!-- hql reference: end -->
