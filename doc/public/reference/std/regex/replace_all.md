<!-- hql reference: std/regex.replace_all -->
# `replace_all`

In [`std/regex`](index.md).

```hql
fn replace_all(text: String, pattern: String, replacement: String) : String
```

`text` with every match of `pattern` replaced by `replacement`, in which
`$1` is the first group:
`"a1b22c" | regex.replace_all("([0-9]+)", "<$1>")` is `"a<1>b<22>c"`.
<!-- hql reference: end -->
