# 0007 — Evaluate a collection literal at its checked element type

Status: backlog

The checker infers a list or set literal's element type from the static types of
its elements. The evaluator infers it again from the runtime types of the values
it built. A view is static, so the two can disagree:

```hql
held : Collection<Int> = [1]
distinct : Collection<Int> = {2}
[held, distinct]
```

The checker accepts this as `List<Collection<Int>>`. At runtime `held` is a
`List<Int>` and `distinct` a `Set<Int>`, which have no common type, so a checked
program fails with `incompatible collection elements`.

[0006](0006-scalar-and-data-union.md) made the case where every element is a
tree hold, by letting the evaluator fall back to `Data`. That repairs a failure
and not the cause. The program above still fails, and where the fallback
applies the runtime tag is broader than the checked type:

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
- `held_together` in `src/evaluation.rs` is removed once that holds.

## Done when

- Both programs above evaluate, to a `List<Collection<Int>>` and a
  `List<Orderable>`.
- For every list and set literal, `Value::type_of` equals the checked type.
