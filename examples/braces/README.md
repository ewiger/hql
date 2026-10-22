# Braces: sets and Data

Braces hold either values or `key: value` entries, and the colon tells them
apart ([HQL-0029](../../doc/proposals/HQL-0029/README.md), the
[set and Data card](../../doc/wiki/hql/set-vs-data.hmd)).

```hql
{}                        // Set<Never>, the empty set
{1, 2, 3}                 // Set<Int>
{:}                       // Data, the empty record
{name: "Alice", age: 42}  // Data
```

| Case | What it shows | Result |
| --- | --- | --- |
| [sets and Data](queries/sets-and-data.hql) | `{}` as an empty `Set<Int>`, a set of three, a record and `{:}` | each printed |
| [empty braces as Data](queries/invalid-empty-braces-as-data.hql) | `d : Data = {}` is refused: `{}` is a `Set<Never>` | refused |

From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/braces/queries/sets-and-data.hql
```
