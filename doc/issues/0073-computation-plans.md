# 0073 — Computation plans: align HQL-0056 with the computation life cycle

Status: backlog
Related: [HQL-0056](../proposals/HQL-0056/README.md) (the sketch),
[HQL-0055](../proposals/HQL-0055/README.md),
[HQL-0005](../proposals/HQL-0005/README.md),
[0063](0063-the-result-of-a-write.md)

HQL-0056 is a sketch. Its one firm idea is that a computation plan is a DAG,
returned by the planning phase. It also names `OpPlan<T>` and `OpResult<T>`
in a new module `core.computation`. None of it is aligned with the
computation life cycle yet, so it does not form a complete model.

## Work

- Write down the computation life cycle phase by phase (check, plan,
  execute, report) and the value each phase gives.
- Fit the DAG plan, `OpPlan<T>`, `OpResult<T>`, `Aute` and `Step` into that
  life cycle as one model.
- Settle the open questions listed in HQL-0056, then rewrite it as a
  specification.

## Done when

- HQL-0056 has a Specification section and no "To be reworked" note.
- HQL-0055 and HQL-0056 agree on `Step`, `Aute` and the planning phase.
