<!-- hql reference: std/regex.captures -->
# `captures`

In [`std/regex`](index.md).

```hql
fn captures(text: String, pattern: String) : Option<List<String>>
```

The first match of `pattern` in `text` and its groups, whole match first,
or `none`; a group that took no part in the match is `""`:
`"a" | regex.captures("(a)|(b)")` is `["a", "a", ""]`.
<!-- hql reference: end -->
