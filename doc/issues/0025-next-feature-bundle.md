# 0025 — Next feature bundle: generics, aliases, reductions and the open ends of the proposals

Status: doing (on `feature/0025-next-bundle`)

The parts of the proposals that are specified, unbuilt, and need no decision,
taken in dependency order. Each feature names the record that specifies it;
nothing here restates it.

## Features

1. **Check-time warnings** — a checker that reports a warning beside a type,
   without failing. Needed by 2 and 6.
2. **The naming lint** — [HQL-0011](../proposals/HQL-0011/README.md): a
   warning at every name a program introduces that breaks its kind's casing,
   and a test that every registered step and every `std/` name passes. The
   sum-type constructor rule is that record's open question and is not in
   scope.
3. **Generic functions** — [0022](0022-generic-functions.md): rigid type
   variables, instantiation at a call, bounds.
4. **`:=` aliases** — [HQL-0015](../proposals/HQL-0015/README.md): the sugar,
   its inferred signature in `hql check`, and the shorthands it defines. Needs
   3. A sugared parameter that is a type is that record's open question and is
   not in scope.
5. **The reduction vocabulary** — [HQL-0014](../proposals/HQL-0014/README.md):
   an operator in function position, `reduce(0, +)`; binary `min` and `max`;
   the combiners `merge`, `union` and `concat` and their `_all` forms;
   `product` and `average`; `[] | sum` is `0`.
6. **The open ends of type interpretation** —
   [HQL-0016](../proposals/HQL-0016/README.md): a warning for an `is(U)` that
   is always true, `is` over a bare constructor, a narrowing `filter` over an
   `Option`, and a refused annotation that names `of_type` or the constructor.
7. **Constructors as operations** — [HQL-0018](../proposals/HQL-0018/README.md):
   a constructor passed as a function, `map(Set)`, and an upper-case name kept
   for constructors.
8. **Graph construction** — [HQL-0019](../proposals/HQL-0019/README.md):
   `Graph(nodes, edges)` and `Graph(edges)`, and `graph = vault.graph`.
9. **The vault's fields** — [HQL-0020](../proposals/HQL-0020/README.md):
   `vault.name`, `vault.root` and `vault.graph`, and a bare `vault` without a
   vault naming `--vault`.
10. **Small diagnostics** — a finite-input error naming `collect(n)` as well
    as `take(n)` ([HQL-0006](../proposals/HQL-0006/README.md)); `collect(for =
    …)` saying time windows are not implemented
    ([HQL-0007](../proposals/HQL-0007/README.md)); an unknown `renderer =` refused at
    check time ([HQL-0017](../proposals/HQL-0017/README.md)).
11. **The proposals state what is true** — every proposal's status line, and
    the text the bundles since 0.1.0 made stale, brought to the implementation;
    HQL-0008 marked superseded by HQL-0017.

## Not in this bundle

- Record values ([0016](0016-sql-parity-analysis.md)), and with them joins
  (HQL-0013), `Table` (HQL-0017), record constructors (HQL-0018) and the join
  form of `Graph` (HQL-0019).
- Data sources (HQL-0012) and `--from` (issue
  [0015](0015-jq-parity-analysis.md)).
- Shapes, stages, the planner and execution backends (HQL-0004, HQL-0005).
- The graph example corpus (HQL-0003), the block form of `tee` (HQL-0010) and
  live sources (HQL-0006), each waiting on its record's open questions.

## Progress

- [ ] 1. Check-time warnings
- [ ] 2. The naming lint
- [ ] 3. Generic functions
- [ ] 4. `:=` aliases
- [ ] 5. The reduction vocabulary
- [ ] 6. The open ends of type interpretation
- [ ] 7. Constructors as operations
- [ ] 8. Graph construction
- [ ] 9. The vault's fields
- [ ] 10. Small diagnostics
- [ ] 11. The proposals state what is true

## Done when

- Every feature above is ticked, or its remaining scope is moved to a card.
- `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`
  and `cargo fmt --check` are clean.
- `CHANGELOG.md` has the release section written and `Cargo.toml` names the
  version, ready to tag from `main`.
