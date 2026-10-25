<!-- hql reference: std/string.chr -->
# `chr`

In [`std/string`](index.md).

```hql
fn chr(self: Int) : Option<String>
```

The one-character text of the Unicode code point `self`, or `none` for
an `Int` that is none: `97 | chr` is `"a"`. The receiver is an `Int`, so
it is written.
<!-- hql reference: end -->
