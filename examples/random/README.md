# Random: samples, shuffles, choices, numbers and UUIDs

The cases follow [HQL-0041 part 4](../../doc/proposals/HQL-0041/parts/04-random.hmd).
The `random` module is imported with `import random`. Each draw has two forms:
with a `seed` argument it is pure, a function of the seed, its input and its
call site; without one it is effectful and draws from the evaluation seed,
which `--seed` fixes and which an evaluation otherwise takes from the host's
entropy and reports at `info`. A seeded draw is keyed by its call site, its
position in the program text, so a case run with `hql run` reads its header
lines as part of that position; the cases below state what holds wherever
the call is written.

| Case | What it shows | Result |
| --- | --- | --- |
| [permutation](queries/shuffle-permutation.hql) | a seeded `shuffle` reorders its input and keeps every element | `[1, 2, 3, 4, 5]` once sorted |
| [unseeded shuffle](queries/shuffle-unseeded.hql) | `shuffle` without a seed draws from the evaluation seed | `true` |
| [order](queries/sample-order.hql) | `sample` keeps input order and draws each element once | `[true, true]` |
| [set](queries/sample-set.hql) | a `Set` built in two orders gives one sample | `true` |
| [whole](queries/sample-whole.hql) | `n` above the size gives the whole input | `["c", "a", "b"]` |
| [unseeded](queries/sample-unseeded.hql) | an unseeded `sample` has `n` elements | `3` |
| [empty choice](queries/choice-empty.hql) | `choice` over an empty sequence | `none` |
| [version 5](queries/uuid-v5.hql) | `uuid(name, namespace)` against RFC 9562's vector | `"2ed6657d-…"` |
| [version 7](queries/uuid-v7-order.hql) | the UUIDs of one evaluation sort in draw order | `[true, true]` |
| [negative sample](queries/invalid-negative-sample.hql) | `sample(-1)` | an evaluation error |
| [inverted bounds](queries/invalid-random-int.hql) | `random_int(6, 1)` | an evaluation error |
| [not imported](queries/not-imported.hql) | `shuffle` without `import random` | a check error |

From the repository root:

```sh
cargo build --locked
target/debug/hql --seed 7 run examples/random/queries/sample-unseeded.hql
```
