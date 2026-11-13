<!-- hql reference: std/string.lines -->
# `lines`

In [`std/string`](index.md).

```hql
fn lines(self) : List<String>
```

The lines of `self`, without their line breaks; a final line break
opens no empty line: `string.lines("a\nb\n")` is `["a", "b"]`.
<!-- hql reference: end -->
