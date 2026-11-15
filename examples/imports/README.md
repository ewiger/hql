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
| [from alias](queries/from-alias.hql) | `from stats import mean as arithmetic_mean` | `2.5` |
| [star except](queries/from-star-except.hql) | `* except (variance, stdev)`, the prelude's `variance` unambiguous | `[2.5, 1.25]` |
| [from workspace](queries/from-workspace.hql) | `from shelf import per_day, shout` | `["103", "DUNE!", "EMMA!"]` |
| [type selector](queries/type-selector.hql) | `from math import Int.abs as int_abs` | `3` |
| [collection receivers](queries/collection-receivers.hql) | `from stats import Collection.*` | `[2.5, 2.5]` |
| [receiver prefix](queries/receiver-prefix.hql) | `String.* as String.bar_*` | `["DUNE!", "emma"]` |
| [import a type](queries/import-a-type.hql) | `from encoding import Hex, String.decode` | `hé` |
| [receiver alias](queries/receiver-alias.hql) | `String.shout as String.yell`: a receiver, and a function | `["DUNE!", "EMMA!"]` |

## Refused

| Case | What it shows | Result |
| --- | --- | --- |
| [qualified only](queries/invalid-qualified-only.hql) | a bare `mean` after `import stats` | a name error naming `stats.mean(…)` and `from stats import mean` |
| [no module name](queries/invalid-from-binds-no-module.hql) | `stats.median` after `from stats import mean` | a name error |
| [misspelt](queries/invalid-misspelt.hql) | `from stats import meen` | a name error at the import |
| [private](queries/invalid-private.hql) | `from shelf import pad`, a private function | a name error at the import |
| [selects nothing](queries/invalid-selects-nothing.hql) | `from stats import List.*`, whose receivers are over `Collection` | a name error at the import |
| [receiver type changed](queries/invalid-receiver-type-changed.hql) | `String.shout as Text.shout` | an import error |
| [group without a pattern](queries/invalid-group-without-pattern.hql) | `* as statistics` | an import error |
| [ambiguous](queries/invalid-ambiguous.hql) | `per_day` from both `shelf` and [`ledger`](lib/ledger/mod.hql) | a name error where it is used |
| [function alias](queries/invalid-function-alias-no-receiver.hql) | `String.shout as yell`, then `"Dune".yell()` | a name error: `yell` is a function only |

From the repository root:

```sh
cargo build --locked
target/debug/hql --modules examples/imports/lib run examples/imports/queries/from-workspace.hql
```
