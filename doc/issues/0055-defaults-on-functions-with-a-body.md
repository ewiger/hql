# 0055 — A parameter default on a function with a body

Status: todo
Related: [HQL-0038](../proposals/HQL-0038/README.md),
[`src/parser.rs`](../../src/parser.rs),
[Declaring a function](../public/learn/04-functions/declaring.md)

HQL-0038, "What a function is", states that a parameter MAY have a default
value, `limit: Int = 0`, and its grammar admits `("=" expression)?` on every
`param`. The parser refuses a default on any function that has a body:

```hql
fn per_week(per_day: Int = 40) : Int = per_day * 7
```

```text
syntax failure: a default belongs to an external declaration; a function with a body takes every argument it declares
```

A default is accepted only on an external (native) declaration. The book's
[Declaring a function](../public/learn/04-functions/declaring.md) page leaves
defaults out until this is resolved.

## Work

1. Decide between the two: admit a default on a function with a body, as
   HQL-0038 states, or narrow HQL-0038 and the grammar in
   `doc/models/behavior/expressions.md` to external declarations.
2. If admitted: `src/parser.rs` drops the refusal; the checker types the
   default against its parameter and fills an omitted trailing argument at
   the call; an alias, a hole and a receiver call over such a function are
   covered.
3. Examples under `examples/functions/`: a default left out and given, a
   default of the wrong type refused, a required parameter after a defaulted
   one refused.
4. The book: a "Default values" section in
   [Declaring a function](../public/learn/04-functions/declaring.md).
