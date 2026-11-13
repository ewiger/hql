<!-- hql reference: std/regex.replace_first -->
# `replace_first`

In [`std/regex`](index.md).

```hql
fn replace_first(text: String, pattern: String, replacement: String) : String
```

`text` with the first match of `pattern` replaced by `replacement`, in
which `$1` is the first group:
`"a1b22c" | regex.replace_first("[0-9]+", "#")` is `"a#b22c"`.
<!-- hql reference: end -->
