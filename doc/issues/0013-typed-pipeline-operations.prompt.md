# Prompt — issue 0013

Paste the block below into a new session, in this repository.

---

Carry out [issue 0013](0013-typed-pipeline-operations.md): make HQL pipelines
general typed programs, with `Stream<T>`, batching, aggregation and
presentation, rather than one MapReduce cycle.

Read `doc/memory/` first, as `CLAUDE.md` requires. Then read, in this order:
the issue, [HQL-0005](../proposals/HQL-0005/README.md) (vocabulary),
[HQL-0006](../proposals/HQL-0006/README.md) (`Stream<T>`),
[HQL-0007](../proposals/HQL-0007/README.md) (`collect` and `emit`),
[HQL-0008](../proposals/HQL-0008/README.md) (presentation), the reduce side of
[HQL-0004](../proposals/HQL-0004/README.md), and
[queries](../models/behavior/queries.md), which holds the collection hierarchy
and traversal carrier this builds on.

Do not start until the four new records are `accepted`. If an Open Question in
one of them is still open, stop and ask me; do not decide it in code.

## The words

Use these terms and no others, in code, comments, docs and commit messages:

- **Pipeline** — an ordered composition of steps.
- **Step** — one invocation at one position in a pipeline.
- **Operation** — the semantics a step invokes: `map`, `filter`, `collect`,
  `emit`, `group`, `reduce`.
- **Operator** — syntax: `|`, `+`, `:`, `->`. `map` is not an operator.
- **Presentation** — `table`, `graph`, `markdown`, `json`, `text`. Not
  "visualization", not "representation".
- **Stage** — only a planned execution unit. Never a synonym for step.

## What is being built

- `map : Stream<T> -> Stream<U>` and `filter : Stream<T> -> Stream<T>` stay
  distinct. Do not express filtering as `map` to `Option` and back.
- `collect(n) : Stream<T> -> Stream<List<T>>` batches; it is not terminal and
  never buffers more than `n`. Represent its argument as a window (count now,
  duration later) so `collect(for = 5s)` is a field, not a new node.
- `emit : Stream<Seq<T>> -> Stream<T>` is its inverse. Do not name it `flatten`.
- Materialization is `list`, not `collect`.
- `group` and `reduce` take HQL-0004's signatures, over finite inputs only. On
  a `Stream<T>` they are type errors whose message names `take` and `collect`.
  Do not give `group` a stream signature; that is deliberately undecided.
- Presentations are registered apart from operations, and must be last.

## Order

Follow the seven commits in the issue. The first is a rename with no behaviour
change; prove it by running the suite with no test edited. Every later commit
updates grammar, type checking, execution tree, tests, examples and the owning
documentation together — never code now and docs later.

## Watch for

- `Seq<T>` becomes a subtype of both `Collection<T>` and `Stream<T>`. Where the
  input is a `Seq`, the existing collection signature must win, so no current
  `expected-type` in the corpus changes. If one does, stop.
- The examples in the issue use lambdas (`e => e.active`), not `_.active`; HQL
  has no placeholder syntax, and adding one is not part of this work.
- Function names are snake_case.
- `graph` is currently a projection returning `Graph`. Splitting it follows
  HQL-0008's resolved name for the projection; check every pipeline in
  `examples/` that continues after `graph`.

## Before you report it done

- The Done when list in the issue is complete, item by item.
- `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`
  and `cargo fmt --check` are clean.
- No document describes an implemented operation as proposed or future work.
- Move the card to `done` in [kanban.yaml](kanban.yaml), and update issue 0012's
  sequence to drop the reduce side it no longer owns.
