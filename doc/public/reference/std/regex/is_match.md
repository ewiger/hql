<!-- hql reference: std/regex.is_match -->
# `is_match`

In [`std/regex`](index.md).

```hql
fn is_match(text: String, pattern: String) : Bool
```

Whether `pattern` matches somewhere in `text`:
`"a1b22c" | regex.is_match("[0-9]+")` is `true`.
<!-- hql reference: end -->
