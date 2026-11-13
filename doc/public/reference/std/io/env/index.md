<!-- hql reference: std/io/env -->
# `std/io/env`

```hql
module io.env
```

What a program reads from the invocation that runs it: environment
variables and named arguments. The environment is read through a grant:
a program run alone is granted every variable; a workspace's `hql.toml`
narrows the grant to `env = ["NAME", "PREFIX_*"]`, a `*` matching any
suffix; `hql render` grants none. A variable that is unset, withheld by
the grant, or whose value is not UTF-8 reads as absent, and the three
are indistinguishable. Arguments are the `--arg name=value` pairs of the
invocation, always granted. Each operation reads a value fixed for the
whole evaluation. A program imports the module, `import io.env`, and
reaches it as `env`.

- [`var`](var.md) — The value of the environment variable `name`, or `none` where it is unset, withheld by the grant, or not UTF-8: `env.var("API_TOKEN") | unwrap_or("")`.
- [`vars`](vars.md) — Every granted environment variable that is set and UTF-8, by name.
- [`arg`](arg.md) — The value of the invocation's `--arg name=value`, or `none` where no `--arg` names it.
- [`args`](args.md) — Every `--arg name=value` of the invocation, by name.
<!-- hql reference: end -->
