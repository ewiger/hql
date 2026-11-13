<!-- hql reference: std/regex.find -->
# `find`

In [`std/regex`](index.md).

```hql
fn find(text: String, pattern: String) : Option<String>
```

The first match of `pattern` in `text`, or `none`:
`"a1b22c" | regex.find("[0-9]+")` is `"1"`.
<!-- hql reference: end -->
