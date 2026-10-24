# HQL-0015: `:=` is function alias syntax sugar

- **Status**: implemented
- **Created**: 2026-10-18
- **Source**: [`reduce` as the aggregation primitive](../HQL-0014/README.md)

## Abstract

`name(params) := fragment` defines a function whose input is the pipeline
value. It is syntax sugar for the `fn` declaration of
[issue 0009](../../issues/0009-function-type-and-constructor.md):
`top(n) := sort | take(n)` is
`fn top<T>(xs: Collection<T>, n: Int) : List<T> where T : Orderable = xs | sort | take(n)`,
and `cards | top(5)` calls it. The checker writes the types the sugar leaves
out, by composing the signatures of the steps in the fragment, and reports the
signature it wrote. Where those signatures do not determine a type — a field
read on an element no step types — the definition is refused, and the program
annotates the input or writes `fn` in full. A sugared definition is an
ordinary function: its arguments are evaluated once, its errors are reported
at its definition, and a planner inlines it as it inlines any pure function.
`=` keeps its meaning, binding the value of an expression. The standard
library and programs both define functions this way.

## Motivation

- A shorthand such as `sum` is admitted only when its longhand is written in
  HQL ([HQL-0014](../HQL-0014/README.md)). `sum := reduce(0, +)` writes the
  longhand as the definition.
- A program repeats fragments — the same filter and sort before every query —
  and needs to name one. A function does that, and HQL needs functions anyway;
  a second mechanism beside them would be two names for one thing.
- A full `fn` signature for a two-step fragment repeats what the steps'
  signatures already say. The sugar keeps the strictness — every function has
  a complete signature — without the program writing what the checker can.

## Non-goals

- **No `fn` design.** The declaration form, the function type and function
  values are issue 0009's. This record adds a shorter spelling of one case.
- **No structural types.** HQL is nominal. The checker does not invent a bound
  from a field a lambda reads; a program states it.
- **No recursion through the sugar.** A sugared definition MUST NOT refer to
  itself, because its signature is computed from its body. A recursive
  function is written with `fn` and a full signature.

## Specification

### 1. What the sugar means

```text
sugar    ::= ident params? ":=" body
params   ::= "(" param ("," param)* ")"
param    ::= ident (":" type)?
body     ::= fragment | expression
fragment ::= step ("|" step)*
```

```text
name(p1, …, pk) := fragment
    ≡  fn name<…>(xs: I, p1: P1, …, pk: Pk) : R = xs | fragment

name(xs: I, p1, …, pk) := expression
    ≡  fn name<…>(xs: I, p1: P1, …, pk: Pk) : R = expression
```

- With a fragment body, the input is implicit: the function's first parameter
  is the pipeline value, and `x | name(a)` calls `name(x, a)`.
- With an expression body, the first parameter names the input and MUST be
  annotated. This is the form for a body whose element type no step fixes.
- `x | name(a)` and `name(x, a)` are the same call, as for every step.

### 2. Which types the checker writes

The checker writes a type only where a signature determines it.

- **The result type** is always written by the checker: it is what the
  fragment's last step returns, composed forward through each step's
  signature, as the checker types any pipeline.
- **A parameter's type** is its annotation, or the type the signature of the
  step it is passed to requires. `n` in `take(n)` is `Int`.
- **The implicit input's type** is the most general type every step accepts,
  with the bounds the steps state. A fragment starting with `sort` gives
  `xs: Collection<T> where T : Orderable`.
- **A literal fixes its type.** `0` in `reduce(0, +)` is an `Int`, so
  `sum := reduce(0, +)` is a sum over integers; a sum over `Float` is a second
  definition, or one `fn` with a bound once a numeric contract exists.
- A definition whose types are not all determined MUST be refused, naming the
  expression whose type is unknown and saying to annotate the input or write
  `fn`. A field read on a generic element is the common case: `c.name` names
  no type until the element is one with a `name`.

| Sugar | The function it defines |
| --- | --- |
| `first := take(1)` | `fn first<T>(xs: Seq<T>) : List<T>` |
| `top(n) := sort \| take(n)` | `fn top<T>(xs: Collection<T>, n: Int) : List<T> where T : Orderable` |
| `sum := reduce(0, +)` | `fn sum(xs: Seq<Int>) : Int` |
| `by_name(n) := sort(by = c => c.name) \| take(n)` | refused: `c.name` on an element no step types |
| `by_name(xs: Seq<Doc>, n) := xs \| sort(by = c => c.name) \| take(n)` | `fn by_name(xs: Seq<Doc>, n: Int) : List<Doc>` |

### 3. The signature is visible

Every function has a complete signature, whether the program wrote it or the
checker did.

- `hql check` MUST report the signature of every sugared definition in the
  program it checks.
- `hql builtins` MUST list the functions `lib/std/` defines with `:=` by their full
  signatures, beside the steps.
- A use is checked against that signature, so an error at a use names the
  function and its parameter, never a step inside its body.

### 4. It is a function

- The body is checked once, at the definition. A malformed or ill-typed body is
  an error there.
- Arguments are evaluated once, as for any call. A parameter used twice in the
  body does not evaluate its argument twice.
- A name in the body that is not a parameter resolves where the function is
  defined, as for any function.
- A planner MAY inline a pure function, sugared or not, and sees its body when
  it does: `xs | sum` is planned as `xs | reduce(0, +)`, with the shape
  HQL-0014 gives that fold.
- A sugared definition is visible after its statement, follows
  [naming](../../wiki/hql/naming.hmd) — `snake_case`, for it and its
  parameters — and MAY shadow a step, with a warning when it shadows a core
  step.

### 5. Which shorthands are admitted

An operation may have a shorter name for one specific case of it.

- A shorthand MUST have a longhand written in HQL, and the relationship MUST be
  stated where the operations are owned, in `lib/std/`. A core shorthand stays a
  host step, and its longhand is written beside its signature in
  `lib/std/collections/mod.hql`.
- Two names at the same level of generality, where neither is a case of the
  other, are not a shorthand and MUST NOT both be admitted. Unary `count(xs)`
  for `size(xs)` is the one kept alias.
- The relationship is what is admitted, not any step in it: a step that exists
  only to make a longhand writable is an ordinary step with its own home.

The shorthands so far, longhand on the left:

```text
map(f) | emit                           flat_map(f)
reduce(0, +)                            sum
reduce(min), reduce(max)                min, max
map(x => 1) | reduce(0, +)              size
filter(x => x == v) | size              count(v)
count(v) > 0                            contains(v)
filter(x => x | is(U))                  of_type(U)
match x { U _ => true, _ => false }     x | is(U)
match b { true => a, false => c }       a given b otherwise c
```

The first six are host steps whose longhands `lib/std/collections/mod.hql` states
beside their signatures; `of_type` takes a type argument, as [HQL-0016](../HQL-0016/README.md) specifies. `contains`, `is` and
`given … otherwise` are shorthands of an expression rather than of a
fragment, so they are stated beside their operations instead.

## Backwards Compatibility

Additive. `:=` is new syntax and no earlier program uses it. The grammar
notation in [the grammar](../../models/behavior/expressions.md) writes its
rules with `::=`, so the notation and the language do not share a symbol.

## Reference Implementation

- `src/lexer.rs`, `src/parser.rs` — the `:=` token and the sugared definition
- `src/checker.rs` — desugaring to the `fn` declaration of issue 0009, the
  signature inference of *Which types the checker writes*, and its report
- `src/builtins.rs` — the full signatures of `lib/std/`'s sugared functions
- `lib/std/collections/mod.hql` — the longhands of the core shorthands of *Which
  shorthands are admitted*, beside their signatures
- `doc/models/behavior/expressions.md` — the sugar rule, and `::=` for the
  notation

## Test Plan

- each row of the table in *Which types the checker writes* defining the function it states, and
  `hql check` reporting that signature
- `xs | sum` and `xs | reduce(0, +)` giving the same value
- `top(5)` equal to `sort | take(5)`; `top()` and `top(5, 6)` refused naming
  `top`; `top("five")` refused naming `n`
- `by_name(n)` refused naming `c.name`, and accepted with the input annotated
- a self-referring sugared definition refused, saying to write `fn`
- a body that reads an outer binding keeping that binding's value when the use
  site binds the same name
- a parameter used twice evaluating its argument once

## Changelog

- 2026-10-18: drafted
- 2026-10-19: implemented; `:=` definitions, their inferred signatures reported
  by `hql check`, and `x | f` over a bound function are built, and the core
  shorthands are host steps whose longhands `lib/std/collections/mod.hql` states
