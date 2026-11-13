<!-- hql reference: std/string.fmt -->
# `fmt`

In [`std/string`](index.md).

```hql
fn fmt(self, args: ...Any) : String
```

The template `self` with each `{}` replaced by the text of the next of
`args`, in order: `fmt("{} of {}", 1, some(2))` is `"1 of 2"`. `...Any`
takes a value of any type in each position. A count of `args` other than
the count of `{}` is an evaluation error. Not `format`, which encodes a
value in a `Format`.
<!-- hql reference: end -->
