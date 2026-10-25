# 0007 — Evaluate a collection literal at its checked element type

Status: done

Resolved by [0011](0011-runtime-architecture-refactoring.md). The evaluator now
consumes typed execution IR, preserving checked element types for literals,
constructors, and `map`, including empty results. Regression coverage lives in
`tests/collections.rs`.

## Original failure

The checker inferred a list or set literal's element type from the static types
of its elements. The evaluator inferred it again from runtime values. A view is
static, so the two could disagree:

```hql
held : Collection<Int> = [1]
distinct : Collection<Int> = {2}
[held, distinct]
```

The checker accepts this as `List<Collection<Int>>`. At runtime `held` is a
`List<Int>` and `distinct` a `Set<Int>`, which have no common type, so a checked
program fails with `incompatible collection elements`.

Since [0006](0006-scalar-and-data-union.md), unrelated trees have `Data` as
their common type, so a literal whose elements are all trees always evaluates.
That hides the cause where it applies and leaves it elsewhere. The program
above still fails, and where trees are held as `Data` the runtime tag is broader
than the checked type:

```hql
first : Orderable = 1
second : Orderable = "owl"
[first, second]
```

This is checked as `List<Orderable>` and evaluates to a value whose `type_of`
is `List<Data>`.

## Scope

- The evaluator takes a literal's element type from the checker rather than
  inferring it a second time. The likely shape is a table from a literal's span
  to its checked type, produced by `check` and read by `evaluate`.
- The same holds for `map`, which re-infers its result's element type from the
  mapped values in `src/extensions/collections.rs`.

## Done when

- Both programs above evaluate, to a `List<Collection<Int>>` and a
  `List<Orderable>`.
- For every list and set literal, `Value::type_of` equals the checked type.
