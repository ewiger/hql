# Imports: qualified names, `from`, type selectors and aliases

The cases follow [HQL-0060](../../doc/proposals/HQL-0060/index.hmd). `import m`
binds the module name only. `from m import …` binds functions unqualified,
selects receivers by the type of their `self`, and renames with `as`. A
standard module and a workspace module are imported the same way. The
workspace module is [`shelf`](lib/shelf/mod.hql), a module `for String`.

| Case | What it shows | Result |
| --- | --- | --- |
| [qualified extension](queries/qualified-extension.hql) | `import stats`: `stats.mean`, a qualified step, a receiver call | `[2.5, 2.5, 2.5]` |
| [qualified workspace](queries/qualified-workspace.hql) | `import shelf`: the same three forms | `["DUNE!", "EMMA!", "103"]` |
| [from names](queries/from-names.hql) | `from stats import mean, median` | `[2.5, 2.5]` |
| [from alias](queries/from-alias.hql) | `from stats import mean as average` | `2.5` |
| [star except](queries/from-star-except.hql) | `* except (variance, stdev)`, the prelude's `variance` unambiguous | `[2.5, 1.25]` |
| [from workspace](queries/from-workspace.hql) | `from shelf import per_day, shout` | `["103", "DUNE!", "EMMA!"]` |
| [type selector](queries/type-selector.hql) | `from math import Int.abs as int_abs` | `3` |
| [collection receivers](queries/collection-receivers.hql) | `from stats import Collection.*` | `[2.5, 2.5]` |
| [receiver prefix](queries/receiver-prefix.hql) | `String.* as String.bar_*` | `["DUNE!", "emma"]` |
| [import a type](queries/import-a-type.hql) | `from encoding import Hex, String.decode` | `hé` |

From the repository root:

```sh
cargo build --locked
target/debug/hql --modules examples/imports/lib run examples/imports/queries/from-workspace.hql
```
