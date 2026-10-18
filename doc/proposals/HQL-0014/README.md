# HQL-0014: `reduce` as the aggregation primitive, and `merge` as a combiner

- **Status**: drafted
- **Created**: 2026-10-18
- **Source**: [MapReduce: pipeline steps, stages and execution backends](../HQL-0004/README.md)

## Abstract

Every aggregation in HQL is a `reduce`, and every named aggregation is an alias
of one. `sum` is the shorthand for `reduce(0, +)`: a fold, as
[HQL-0004](../HQL-0004/README.md) defines it, whose combiner is `+`. A
**combiner** is an ordinary binary operation on two values of one type — `+`,
`min`, `merge`, `union`, `concat` — and the host knows its algebraic
properties. `merge(a, b)` combines two maps, the right one winning a conflicting
key, and `maps | reduce({}, merge)` is the same operation over a sequence of
maps. A fold whose combiner is one of these has shape `Reduction` and may be
computed from partial results; a fold with a lambda stays a `Barrier`, since
nothing is known about it. `reduce` without an initial value folds from the
first element and returns `Option<T>`, which is what `min` and `max` alias. An
order-sensitive combiner, such as `merge` or `concat`, requires a `Seq`; an
order-independent one, such as `+` or `union`, accepts any collection. HQL has
no separate aggregation subsystem.

## Motivation

- HQL needs `sum`, `min`, `max`, `average`, `count`, a histogram, a top-k and
  more. Giving each its own execution concept would duplicate what a fold
  already is, and each would need its own rule for parallel execution.
- HQL-0004 makes every fold a `Barrier`, because a function the program supplies
  cannot be assumed associative. That is right for a lambda and too
  conservative for `+`, which every executor can split. The difference is a
  property of the combiner, and it needs a place to be stated.
- Combining two maps is useful on its own — defaults overridden by a vault's
  configuration, metadata assembled from two layers — and it is also the
  combiner that reduces many maps to one. One operation should serve both.

## Goals

- one aggregation mechanism, `reduce`, with each named aggregation an alias of
  a fold written in HQL
- `merge` as a binary operation and as a combiner, with deterministic conflicts
- a statement of when a fold may be split, carried by its combiner
- a defined result over empty input for every aggregation

## Non-goals

- **No combiners declared in HQL.** A program folds with any lambda; giving a
  program-defined operation properties the planner trusts is a later proposal,
  since a property the program asserts is one the host cannot check.
- **No exposed algebra.** Associativity, identity and order sensitivity are
  host metadata. The word *monoid* is not HQL syntax, and no program writes a
  law.
- **No configurable conflict strategies yet.** `merge` is right-biased; a
  `merge` that takes a conflict function is later work.
- **No deep merge.** `merge` combines top-level keys only.

## Specification

### 1. Terms

- **combiner**: a binary operation the host provides that takes two values of
  one type and returns one of that type: `+`, `*`, binary `min` and `max`,
  `merge`, `union`, `concat`.
- **fold**: `reduce` with an initial value and a combining function.
- **known fold**: a fold whose function is a combiner, named directly — not a
  lambda that calls one.
- **accumulator**: the value a fold carries between elements. For a known fold
  it has the element type; for a fold with a lambda it may differ.

### 2. `reduce`

`reduce` folds a sequence, from an initial value or from its first element.

```text
reduce<T, R>(xs: Seq<T>, initial: R, by: (R, T) -> R)  : R          HQL-0004
reduce<T>(xs: Seq<T>, by: (T, T) -> T)                 : Option<T>  from the first element
```

```hql
values | reduce(0, +)
values | reduce(max)
maps   | reduce({}, merge)
values | reduce(initial = 0, by = (acc, x) => acc + x * x)
```

- The first form is HQL-0004's left fold, unchanged in meaning.
- The second form folds from the first element: `by(by(x0, x1), x2)` and so on.
  It returns `none` over an empty input and `some` of the fold otherwise.
- `by` may be a lambda or a combiner named directly. A combiner in that position
  is resolved by the checker against the combiners the host provides, as
  `of_type(T)` resolves a type; it is not a function value, and an unknown name
  is a name error listing the nearest combiner.
- An operator names its combiner: `+` in `reduce(0, +)` is the addition
  combiner.
- A known fold whose combiner is order-independent (section 5) accepts any
  `Collection<T>`, because its result does not depend on the order it visits
  elements. Every other fold accepts only a `Seq<T>`, and over a `Set` the
  checker refuses it with a message saying to sort first, by HQL-0004's rule.

### 3. Named aggregations are aliases

A named aggregation is the shorthand for a fold written in HQL, defined with
`:=`, the function alias syntax sugar of [HQL-0015](../HQL-0015/README.md).

```text
sum         :=  reduce(0, +)
product     :=  reduce(1, *)
min         :=  reduce(min)
max         :=  reduce(max)
size        :=  map(x => 1) | reduce(0, +)
count(v)    :=  filter(x => x == v) | size
merge_all   :=  reduce({}, merge)
union_all   :=  reduce(Set([]), union)
concat_all  :=  reduce([], concat)
```

- Each shorthand MUST equal its longhand over every input, in value and in the
  result over empty input: `[] | sum` is `0`, `[] | min` is `none`.
- The longhand is written in HQL, which is the rule that admits a shorthand.
- A shorthand is a pure function whose body is its longhand, and a planner
  inlines it, so it sees one fold whichever spelling the program used.
- `count(xs)` stays the kept alias of `size(xs)`.
- `min` and `max` each have two arities: binary `min(a, b)` is the combiner, and
  `xs | min` is the aggregation. They are one name at two arities, as `count` is.

### 4. `merge` combines two maps

`merge` is an ordinary binary operation, and a combiner.

```text
merge<K, V>(left: Map<K, V>, right: Map<K, V>)               : Map<K, V>
merge<K, V>(left: OrderedMap<K, V>, right: OrderedMap<K, V>) : OrderedMap<K, V>
merge(left: Data, right: Data)                               : Data
```

```hql
merge({a: 1, b: 2}, {b: 3, c: 4})            // {a: 1, b: 3, c: 4}
[{a: 1}, {b: 2}, {c: 3}] | reduce({}, merge)  // {a: 1, b: 2, c: 3}
```

- The result holds every key of either input. A key in both takes the value
  from `right`. Merging is right-biased, and that is the one conflict rule.
- Over `OrderedMap`, the result's keys are `left`'s in their positions, then
  `right`'s new keys in theirs; a conflicting key keeps `left`'s position and
  takes `right`'s value. A `SortedMap` is ordered by its keys as always.
- Over `Data`, `merge` applies to two string-keyed maps of trees and combines
  their top-level keys; a value that is itself a map is replaced, not merged.
  Merging a `Data` value that is not a map is an evaluation error naming the
  value's kind, reported through the
  [structured log](../../wiki/hql/error-reporting.hmd).
- As a combiner, `merge` has the empty map as its identity, is associative, and
  is order-sensitive: which value wins depends on which map comes later. So
  `Seq<Map<K, V>> | reduce({}, merge)` is deterministic, and a `Set` of maps is
  refused.

### 5. Combiner properties

A combiner carries properties the host states and the planner reads.

```text
Combiner
    identity           e with combine(e, x) = x = combine(x, e), when there is one
    associative        combine(combine(a, b), c) = combine(a, combine(b, c))
    commutative        combine(a, b) = combine(b, a)
    order_sensitive    not commutative: input order changes the result
```

| Combiner | Over | Identity | Associative | Commutative | Input |
| --- | --- | --- | --- | --- | --- |
| `+` | numbers | `0` | yes | yes | any collection |
| `*` | numbers | `1` | yes | yes | any collection |
| `min`, `max` | `Orderable` | none | yes | yes | any collection |
| `union` | `Set<T>` | the empty set | yes | yes | any collection |
| `merge` | maps | the empty map | yes | no | `Seq` |
| `concat` | `Seq<T>` | the empty list | yes | no | `Seq` |

- These properties are host metadata, as shape is by HQL-0004. `hql builtins`
  MUST print them beside each combiner, and they MUST NOT be writable in HQL.
- A host MUST NOT declare a property it does not hold. A test per combiner
  checks each declared law over generated inputs.
- Floating-point `+` is associative only up to rounding. A host MUST either
  combine floating-point partials in a fixed order, so that every backend gives
  the same sum, or declare `+` over `Float` not associative.
- `union` here is the combiner on sets. The word also introduces a type in
  `type Data = union {…}`; keywords are recognized by position, so the two do
  not meet.

### 6. Execution shape

Whether a fold may be split follows from its combiner, not from the name
`reduce`.

- A known fold with an associative combiner has shape `Reduction`. An executor
  MAY fold each split of the input without the initial value and combine the
  partial results, applying `initial` once, at the front:
  `initial ⊕ (x0 ⊕ … ⊕ xn)` equals the left fold by associativity, so the split
  is valid for any `initial`, not only the identity.
- For an order-sensitive combiner the splits MUST be contiguous and combined in
  order; for a commutative one they may be combined in any order.
- A fold whose `by` is a lambda keeps shape `Barrier`, even when the lambda only
  calls a combiner. A planner MAY recognize `(a, b) => a + b` as `+`, and MUST
  NOT be required to.
- This refines HQL-0004, which lists `reduce` as a barrier: the shape is a
  property of the checked step, and the step includes its combiner. `size`,
  `count` and `contains` are reductions there already for the same reason.

```text
partition A  -> fold with ⊕ -> partial A ┐
partition B  -> fold with ⊕ -> partial B ├─ initial ⊕ partial A ⊕ partial B ⊕ partial C
partition C  -> fold with ⊕ -> partial C ┘
```

### 7. Aggregations whose accumulator differs

Some aggregations carry a state of another type and finish by projecting it.

- `average` carries a sum and a count, combines both by addition, and finishes
  with their quotient. Over an empty input it returns `none`. Its longhand is a
  fold with a lambda and a projection:

  ```hql
  s = values
      | reduce(initial = {sum: 0.0, count: 0},
               by = (s, x) => {sum: s.sum + x, count: s.count + 1})
  s.sum / s.count
  ```

  That longhand is a `Barrier`, because its function is a lambda. The alias
  `average` is checked to a step the host holds, whose state combines by `+`
  on both fields; that step is a `Reduction`. The two MUST give the same value.
- Later aggregations whose state differs from their element — `variance`,
  `histogram`, `top_k`, `count_by`, `approx_distinct` — follow this pattern: a
  longhand fold written in HQL, and a host step with a known state combiner
  that equals it. An approximate aggregation MUST still be deterministic: the
  same input gives the same answer on every backend.

### 8. Relationship to the other core operations

The core operations differ in what they do to the number of values.

```text
map        N -> N
filter     N -> at most N
flat_map   N -> M
group      N -> groups
reduce     N -> 1
```

- `flat_map(f)` is the shorthand for `map(f) | emit`, by
  [HQL-0007](../HQL-0007/README.md). It is elementwise and does not end a
  stream. Over a finite sequence, `map(f) | reduce([], concat)` gives the same
  `List`, but it is a reduction: it needs every element before it answers, and
  over a stream it is refused. The two are equal in value over finite input and
  different in shape.
- `group` is a partition, not a reduction. It returns a `Set<Group<K, T>>`, and
  aggregation is applied inside each group:

  ```hql
  cards
  | group(by = c => c.metadata.author)
  | map(g => {author: g.key, pages: g.items | map(c => c.metadata.pages) | sum})
  ```

## Backwards Compatibility

Additive. `reduce` without `initial`, the combiners `+`, `*`, `min`, `max`,
`merge`, `union` and `concat` in the `by` position, and the aliases of
section 3 are new. `reduce(initial, by)` with a lambda keeps HQL-0004's meaning
and shape. `size`, `count` and `contains` keep their results. The aliases of
section 3 are among the shorthands [HQL-0015](../HQL-0015/README.md) lists.

## Security Considerations

A fold holds one accumulator, but a combiner such as `union`, `merge` or
`concat` grows it with its input. The run-context bounds of
[HQL-0012](../HQL-0012/README.md) apply to an accumulator as to any held value,
and exceeding one is an error, never a truncated result.

## Deployment / Activation

1. `reduce` without `initial`, and combiners in the `by` position: `+`, `min`,
   `max`.
2. The aliases `sum`, `min`, `max`, checked to the same step as their
   longhands.
3. `merge` over `Map`, `OrderedMap` and `Data`, as a binary operation and as a
   combiner.
4. Combiner properties in `hql builtins`, and the `Reduction` shape for known
   folds.
5. `union`, `concat`, `average`, then the aggregations of section 7, each with
   its own tests.

## Reference Implementation

- `src/extensions/mapreduce.rs` — `reduce` without `initial`, the combiner
  registry and its properties, beside HQL-0004's `reduce` and `group`
- `src/extensions/collections.rs` — `merge`, and the alias steps
- `src/builtins.rs` — the combiner property columns
- `std/collections.hql` — the signatures beside the existing core operations
- `examples/mapreduce/` — `total-pages` with `sum`, and a `merge` of two layers
  of metadata

## Test Plan

Unit tests MUST include:

- each alias equal to its longhand, over inputs and over empty input
- each declared law — associativity, commutativity, identity — over generated
  inputs, for every combiner that declares it
- `merge` with a conflicting key taking the right value, and `OrderedMap` key
  order as section 4 states
- a split fold with a non-identity `initial`, `reduce(10, +)`, giving the
  sequential result
- `reduce({}, merge)` and `reduce([], concat)` over a `Set`, refused with the
  message saying to sort first

Integration tests MUST include:

- a known fold split into partitions and combined giving the same result as one
  sequential pass, for every associative combiner
- the same fold written with a lambda reported as a `Barrier` by
  `hql builtins`-visible shape, and a combiner as a `Reduction`
- `map(f) | reduce([], concat)` equal to `flat_map(f)` over a finite list, and
  refused over a stream
- `average` over an empty input giving `none`

```bash
cargo test
cargo test --test corpus
```

## Open Questions

None.

## Changelog

- 2026-10-18: drafted
