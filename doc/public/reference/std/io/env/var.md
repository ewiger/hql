<!-- hql reference: std/io/env.var -->
# `var`

In [`std/io/env`](index.md).

```hql
effectful fn var(name: String) : Option<String>
```

The value of the environment variable `name`, or `none` where it is
unset, withheld by the grant, or not UTF-8:
`env.var("API_TOKEN") | unwrap_or("")`.
<!-- hql reference: end -->
