# Mighty Types: the language, program by program

Twenty-three pure programs in the manner of a language textbook, each showing
one part of the language that [HQL-0024](../../doc/proposals/HQL-0024/README.md)
specifies: records, unions, options and results, traits and operators, generic
functions, closures, iterators, the standard library, modules written in HQL
and the dataflow operations a dataframe has. From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/mighty-types/queries/01-hello-world.hql
target/debug/hql run --modules examples/mighty-types/lib examples/mighty-types/queries/18-modules.hql
```

Each program's header records its status. A program marked `proposed` is a
design input pinned ahead of the implementation: the harness asserts it does
not check yet, and the day it does the header is promoted to `valid-now`.

| Program | What it shows | Result |
| --- | --- | --- |
| [hello world](queries/01-hello-world.hql) | `fmt` and the program's value | `Hello, world!` |
| [formatted print](queries/02-formatted-print.hql) | `fmt` with several placeholders, `str` of any value | seven strings |
| [primitives](queries/03-primitives.hql) | Int arithmetic, `int` and `float`, `import math`, `abs` and `max` as receivers | `[3, 1, -1, 1024, …]` |
| [strings](queries/04-strings.hql) | The `string` receiver module of the prelude | twelve strings |
| [records](queries/05-records.hql) | Record types, constructors by field name, nested reads | `[12, 4, 3, 1, 1]` |
| [unions](queries/06-unions.hql) | A union of records matched by type, a literal held at the union | `3 shapes, total area 19.0` |
| [option](queries/07-option.hql) | Optional fields, positions past the end, `map`, `and_then`, `unwrap_or` | six strings |
| [result](queries/08-result.hql) | `Result<T, E>`, `ok` and `err` as values and patterns | four strings |
| [traits](queries/09-traits.hql) | `trait`, `impl … for …`, a default body, `T : Shape` | four strings |
| [operators](queries/10-operators.hql) | `impl Add` and `impl Sub` for a declared type | `[4, 6, 2, 2, 1]` |
| [ordering](queries/11-ordering.hql) | `impl Ord` makes a type `Orderable` for `<`, `sort`, `max` | three strings |
| [generics](queries/12-generics.hql) | `Stack<T>` inferred at construction, `Pair<A, B>` swapped | five strings |
| [closures](queries/13-closures.hql) | Capture, composition, repetition, a hole, `+` as a function | `[8, 8, 15, 15, 15, 14]` |
| [iterators](queries/14-iterators.hql) | `range`, a yielding stream, `zip`, `enumerate`, `take_while`, `any`, `all` | ten strings |
| [collections](queries/15-collections.hql) | Set algebra, maps from sequences and `to_map`, `with`, `entries`, `distinct` | eleven strings |
| [pattern matching](queries/16-pattern-matching.hql) | Literals, guards, type patterns over `Data`, nested `some` | FizzBuzz and friends |
| [recursion](queries/17-recursion.hql) | Factorial, gcd, quicksort, primes, Collatz | `[3628800, 21, 123589, 10, 111]` |
| [modules](queries/18-modules.hql) | `import mighty.geometry as geo` from the `lib` root | `[3.0, 12.0, 4.0]` |
| [receiver module](queries/19-receiver-module.hql) | `module for String` written in HQL, used as `"hi".shout()` | four strings |
| [regex](queries/20-regex.hql) | `import regex`: match, find, replace, capture, split | seven strings |
| [dataframe](queries/21-dataframe.hql) | Records grouped, aggregated, sorted and joined | two lines |
| [word count](queries/22-word-count.hql) | Split, group, count, order, take: MapReduce in one pipeline | `the=3 fox=2 brown=1` |
| [stats module](queries/23-stats-module.hql) | A module written in HQL that imports `math` for itself | `[5.0, 4.0, 2.0, 4.5]` |

## The modules under `lib/`

`lib/` is a module root: a namespace boundary whose files are modules named by
their position. `lib/mighty/geometry.hql` is `mighty.geometry`,
`lib/mighty/text.hql` is `mighty.text` and `lib/mighty/stats.hql` is
`mighty.stats`; `mighty/` itself is a namespace directory with no `mod.hql`,
so `import mighty` names nothing. A program names the root through the
`modules:` line of its header, as `hql run --modules examples/mighty-types/lib`
does on the command line and `[modules] roots = ["lib"]` does in a
workspace's `hql.toml`. Files under a module root are not corpus cases.

| Module | Kind | Provides |
| --- | --- | --- |
| `mighty.geometry` | plain | `Circle`, `Rect`, `circle_area`, `rect_area`, `unit_circle`, `scale` |
| `mighty.text` | `module for String` | `shout`, `initials`, `is_blank` |
| `mighty.stats` | plain, imports `math` | `mean`, `variance`, `stddev`, `median` |
