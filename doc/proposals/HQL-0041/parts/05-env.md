# Part 5: `io.env`

- **Depends on**: [the proposal](../README.md) for ownership and the values
  fixed per evaluation; [HQL-0025](../../HQL-0025/README.md) for execution
  contexts; [HQL-0021](../../HQL-0021/README.md) for host grants
- **Needed by**: none

## Scope

What a program reads from the invocation that runs it: environment variables
and named arguments. Python's `os.environ` and `sys.argv` cover the same
ground. HQL-0025 decides how the environment shapes the execution context;
this part decides what a program can read of it.

## Declarations

```hql
// lib/std/io/env/mod.hql
module io.env
import core
import io

effectful fn var(name: String) : Option<String>
effectful fn vars() : Map<String, String>
effectful fn arg(name: String) : Option<String>
effectful fn args() : Map<String, String>
```

- `var` MUST give the variable's value where it is set and granted, and
  `none` otherwise. An unset variable and a withheld one MUST be
  indistinguishable to the program.
- `vars` MUST give every granted variable that is set.
- A variable whose value is not UTF-8 MUST read as `none` and be absent from
  `vars`.
- `arg` and `args` MUST read the `--arg name=value` pairs of the invocation.
  A name given twice MUST be an invocation error before evaluation starts.
  `name` MUST match `[a-z_][a-z0-9_]*`.
- An argument is text. A program converts it, `arg("limit") | and_then(to_int)`,
  as it converts any text.

## Grants

The environment is read only through a grant.

- Standalone, `hql script.hql`, MUST grant every variable.
- A workspace MAY narrow the grant with `env = ["NAME", "PREFIX_*"]` in
  `hql.toml`; a pattern's `*` matches any suffix.
- `hql render` MUST grant no variable. A rendered document is shared, and its
  output must not depend on, or disclose, the renderer's environment.
- Arguments are always granted: the invocation wrote them.

## Examples

```hql
import io.env

// A query parameterised by its invocation:
//   hql query.hql --arg species=owl --arg limit=20
species = env.arg("species") | unwrap_or("owl")
limit   = env.arg("limit") | and_then(to_int) | unwrap_or(10)

source(file("sightings.jsonl"), JsonLines)
| filter(r => r.species == species)
| take(limit)

// A token from the environment, for a network endpoint.
token = env.var("API_TOKEN")
```

## Test plan

- `var` over a set, an unset and a withheld variable; the last two equal.
- `vars` under a narrowed `hql.toml` grant holds only the matching names.
- `hql render` reads no variable, including one the shell sets.
- `--arg` read by `arg` and `args`; a repeated name and a malformed name
  refused before evaluation.
- A non-UTF-8 variable reads as `none`.
