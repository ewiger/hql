<!-- hql reference: std/regex.split -->
# `split`

In [`std/regex`](index.md).

```hql
fn split(text: String, pattern: String) : List<String>
```

The pieces of `text` between matches of `pattern`, in order:
`"a1b22c" | regex.split("[0-9]+")` is `["a", "b", "c"]`.
<!-- hql reference: end -->
