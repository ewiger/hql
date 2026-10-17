# 0018 — Operator synonymy: one name per abstraction level

Status: backlog
Decisions: [HQL-0004](../proposals/HQL-0004/README.md),
[HQL-0006](../proposals/HQL-0006/README.md),
[HQL-0007](../proposals/HQL-0007/README.md)
Related: step 2 and step 5 of [0013](0013-typed-pipeline-operations.md),
[0016](0016-sql-parity-analysis.md), [0017](0017-graph-projection-and-view.md)

HQL's operation vocabulary is growing on three fronts at once: the implemented
core (`hql builtins`), the reduce and stream proposals, and the SQL and jq
parity lists. Each front names operations on its own, so the same operation can
pick up two names, and nothing says which pairs are intended.

The rule this card applies: **specialization is fine; synonymy is not.** Two
names may coexist when one is a named special case of the other and the
relationship is written down (`sum(xs)` is `reduce(xs, 0, +)`). A new name is
not admitted when an existing one already means the same thing.

The one alias HQL keeps on purpose is unary `count(xs)` for `size(xs)`. It is
declared as such in `std/collections.hql` and
[expressions](../models/behavior/expressions.md), it is the only one, and the
rule above governs admissions from here on, not that pair.

## Findings

Checked against `hql builtins`, `std/collections.hql`, the proposals and the
corpus:

| Names | Relationship | Outcome |
| --- | --- | --- |
| `size(xs)` / unary `count(xs)` | a declared alias | keep; the pipes card names it as the one alias |
| `size`, `count(xs, x)`, `contains` | derived: `contains` iff `count > 0`; `size` sums `count` | keep all; the derivation is already in `std/collections.hql` |
| `reduce` / `size`, `count(xs, x)`, future `sum`, `min`, `max` | special reductions of `reduce` | keep; state the tree |
| `filter` / `typed` | `typed(U)` is `filter(x => x is U)` plus a narrowing the checker sees; non-members are dropped, not reported | keep; state it |
| `typed` over a collection / `match` on one value | `typed(U)` is `match` on each element's type, keeping the `U` arm; branching on one value's type is `match` itself | keep both; no single-value `typed` and no reporting `validate`. `match` needs a type pattern (`RelationCard r => …`), and its [card](../wiki/hql/operators/match-op.hmd) example `match c.kind { "ConceptCard" => … }` branches on an authored label, which the type-system card forbids |
| `collect` (HQL-0004) / `list` (HQL-0007) | HQL-0007 renamed materialization to `list`; `collect(n)` batches | one name each; stale text remains |
| `emit` / `flatten` / `flat_map` | HQL-0007 declines `flatten` and names it `emit`; 0016 asks for `flatten` and `flat_map` | `flat_map(f)` is `map(f) \| emit`; decide whether it earns a name |
| `drop` / `skip` | [queries](../models/behavior/queries.md) already names `drop` | one name; never add `skip` |
| `take` / `collect(n)` | first `n` versus batches of `n` | distinct; keep |
| `expand` / `uplinks`, `downlinks` | `Graph` versus `List<Edge>` | distinct types; the one-step contract must say so |
| `collect` the operation / `collect` the reporting mode | different namespaces (pipeline step, `--report` flag) | keep; name the overlap on the error-reporting card |

`filter`/`where`, `take`/`limit`, `map`/`select` and `join`/`select` are not
findings: HQL has no `where`, `limit` or `select`, and the
[design status](../wiki/hql/design-status.hmd) card keeps SQL clauses out.

## Work

1. **State the abstraction tree** where operations are owned: the
   [pipes](../wiki/hql/types/pipes.hmd) card gets a short section naming each
   general operation and its named specializations (`reduce` over `size`,
   `count`, `sum`, `min`, `max`; `filter` over `typed`; `map | emit` over
   `flat_map`), the rule above as the admission test for a new name, and
   unary `count` as the declared alias of `size`.
2. **Finish the `collect` rename.** `examples/mapreduce/README.md` still says
   `collect` materializes a sequence; it reads `list`. HQL-0004's own text is
   covered by 0013's Done when and is not repeated here.
3. **Reconcile 0016 with HQL-0007.** The `CROSS JOIN` and `INNER JOIN` rows
   name `flatten`; they name `emit`. Whether `flat_map` earns a first-class
   name is decided in 0016's step on `flat_map` and `join`, under the rule
   here: it does if the specialization is as common as `sum` is to `reduce`.
4. **`expand` against `uplinks`.** The [graph-type](../wiki/hql/types/graph-type.hmd)
   card states that `expand(depth = 1, direction = "out")` returns a `Graph`
   and `uplinks` returns the edges, so one is not a spelling of the other.
   The `graph` split itself stays in 0017.
5. **Name the `collect` overlap** on the
   [error-reporting](../wiki/hql/error-reporting.hmd) card: the reporting mode
   and the stream operation share a word and nothing else.

## Done when

- The pipes card states the specialization rule, the tree and the `count`
  alias, and every operation in `hql builtins` appears in it exactly once.
- No card, model, proposal or corpus README says `collect` materializes a
  sequence.
- 0016 names `emit`, not `flatten`.
- `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`
  and `cargo fmt --check` are clean.
