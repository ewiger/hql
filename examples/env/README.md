# Env: environment variables and invocation arguments

The cases follow [HQL-0041 part 5](../../doc/proposals/HQL-0041/parts/05-env.md).
The `io.env` module is imported with `import io.env` and reached as `env`.
`env.var` and `env.vars` read the environment through the host's grant:
every variable for a program run on its own, the names `env = [...]` lists
in a workspace's `hql.toml`, and none under `hql render`. An unset variable
and a withheld one both read as `none`. `env.arg` and `env.args` read the
invocation's `--arg name=value` pairs, which are always granted. Every
operation is effectful.

The cases are run with no `--arg`, so each shows the default a program
supplies; the commands below pass arguments.

| Case | What it shows | Result |
| --- | --- | --- |
| [default](queries/arg-default.hql) | an argument not passed, defaulted | `owl` |
| [conversion](queries/arg-convert.hql) | an argument converted to an `Int`, defaulted | `10` |
| [no arguments](queries/args-none.hql) | `args()` without `--arg` | `0` |
| [unset variable](queries/var-unset.hql) | a variable that is not set | `anonymous` |
| [not imported](queries/not-imported.hql) | `var` without `import io.env` | a check error |

From the repository root:

```sh
cargo build --locked
target/debug/hql --arg species=wren run examples/env/queries/arg-default.hql
target/debug/hql --arg limit=20 run examples/env/queries/arg-convert.hql
target/debug/hql --arg a=1 --arg a=2 run examples/env/queries/args-none.hql   # refused, exit 2
```
