# 0012 — MapReduce: pipeline stages and execution backends

Status: backlog
Decision: [HQL-0004](../proposals/HQL-0004/README.md)
Branch: `feat/mapreduce`, off `feat/semantic-search`
Depends on: nothing; [0010](0010-yield-and-bind.md) makes `collect` observable

The work that carries out [HQL-0004](../proposals/HQL-0004/README.md). Read the
proposal first; it holds the design, and this card holds only the sequence and
what "done" means. The example that pins the intended results is already in the
corpus: [`examples/mapreduce/`](../../examples/mapreduce/README.md), with three
queries at `status: proposed`.

## Sequence

Five commits, in this order. The first changes no behaviour, which is what
makes the rest safe.

1. **Shapes.** Every registered `Step` declares a `Shape` beside its `Purity`,
   and `hql builtins` prints it. No evaluator changes.
2. **The reduce side.** `Group<K, T>` as a type constructor with `key` and
   `items`; `reduce`, `group` and `collect` as core steps in
   `src/extensions/mapreduce.rs`. The three proposed corpus cases are promoted
   to `valid-now` in this commit — `proposed_cases_do_not_check_yet` will
   insist.
3. **The backend seam.** A `Backend` trait in `src/backends/`, with
   `localhost` as its only member wrapping today's evaluator; `hql::run` goes
   through it. Prove it is a no-op: the suite passes with no test edited.
4. **Selection.** `[execution] backend` in `hql.toml`, `--backend`,
   `HQL_BACKEND`, and `hql config` reporting which is in force and why. An
   unknown name fails every program and names its source.
5. **Conformance.** A test that runs the whole corpus through every registered
   backend and compares each answer to `localhost`. With one backend it is a
   tautology; it is written now so that the second backend arrives into a test
   rather than writing one.

## Done when

- Every query in `examples/mapreduce/` is `valid-now` and runs from the command
  line as its README prints it.
- `hql builtins` shows a shape for every step, and every step has one.
- `hql config` reports the backend in force and where it came from.
- `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`
  and `cargo fmt --check` are clean.
- The decisions are stated where they are owned: `std/collections.hql` declares
  `Group<K, T>` and the three signatures; `doc/wiki/hql/types/pipes.hmd` says
  what a stage is and what a backend may do; `doc/models/behavior/cli.md`
  documents `--backend` and `[execution]`; `doc/models/domain/architecture.md`
  puts the backend in its diagram and stops saying execution is sequential
  by necessity. Use `doc/memory/` only for anything left undigested.

## Settle before starting

The Open Questions at the end of [HQL-0004](../proposals/HQL-0004/README.md).
They are frozen before its status moves from `drafted` to `accepted`, and the
proposal is accepted before this card is picked up.
