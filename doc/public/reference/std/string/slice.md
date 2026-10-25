<!-- hql reference: std/string.slice -->
# `slice`

In [`std/string`](index.md).

```hql
fn slice(self, start: Int, end: Int) : String
```

The characters of `self` from `start` up to, not including, `end`,
counting from 0; `s[start:end]`. The bounds are clamped to the text, and
a negative bound counts from the end: `"hello" | slice(-3, 10)` is
`"llo"`. `s.slice(range)`, with one argument, is the core trait `Slice`.
<!-- hql reference: end -->
