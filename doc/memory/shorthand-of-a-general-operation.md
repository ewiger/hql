# Shorthand of a general operation

An HQL operation may have a shorter name for one specific case of it. The
specific form is admitted when its longhand can be written in HQL, and the
relationship is stated where the operations are owned. Instances so far:

```text
reduce(0, +)                         sum
reduce(…)                            min, max, size, count(x)
count(x) > 0                         contains(x)
match x { U u => some(u), _ => none }
  | somes                            typed(U)
match b { true => a, false => c }    a given b otherwise c
```

The pattern is the relationship, not any step in it. `somes` is only the
collection step that makes the `typed` longhand writable: `List<Option<T>>` to
`List<T>`, keeping the input's shape. It is designed and not implemented, and
has no home yet on the option-type card.

Two names at the same level of generality, where neither is a case of the
other, are not this pattern and are not admitted. Unary `count(xs)` for
`size(xs)` is the one kept alias.

Home when digested: the pipes card, as step 1 of issue 0018; `somes` on the
option-type card.
