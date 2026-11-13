<!-- hql reference: std/regex.find_all -->
# `find_all`

In [`std/regex`](index.md).

```hql
fn find_all(text: String, pattern: String) : List<String>
```

Every non-overlapping match of `pattern` in `text`, in order:
`"a1b22c" | regex.find_all("[0-9]+")` is `["1", "22"]`.
<!-- hql reference: end -->
