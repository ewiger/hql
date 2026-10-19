# HQL-0018: A type's constructor is an operation

- **Status**: accepted
- **Created**: 2026-10-18
- **Source**: [HQL-0016](../HQL-0016/README.md), its section "Conversion is
  construction"; [HQL-0017](../HQL-0017/README.md)

## Abstract

Every concrete type with a constructor can be used in two forms: called,
`Graph(cards)`, and as a pipeline step, `cards | Graph`. The piped value is the
constructor's first argument, so `x | T(a)` is `T(x, a)`. HQL-0016 states this
for the collection constructors; this record makes it the rule for every type,
whatever its first parameter is: a sequence, a graph, a join's rows or a
single value. A type may have several constructors, told apart by the type of
their first argument. A constructor is declared as a function named after its
type, in the module that declares the type, and returns that type. Because a
constructor builds a value of its type, no lowercase operation may do the same
under a second name: the operations `list`, `graph` and `table` are retired
in favour of `List`, `Graph` and `Table`. The name `graph` stays, as a
binding of the vault's knowledge graph, which is a value and not an
operation.

## Motivation

- HQL-0016 makes `List(xs)` and `xs | List` the conversion between
  collections, and HQL-0017 makes `Graph(xs)` the projection to a graph and
  `Table(xs)` the way to build a table. Three proposals now rely on the same
  convention, and none states it for types in general.
- Lowercase operations that build a value of one type, such as `list` and
  `graph`, are second names for that type's constructor, which the rule of
  [issue 0018](../../issues/0018-operator-synonymy-audit.md) does not admit.
- The [expressions](../../models/behavior/expressions.md) model leaves open
  whether a constructor may stand where something other than a call is written.
  A pipeline step and a function argument are the two places a program needs
  one.

## Specification

### 1. Which types have constructors

- A concrete type MAY declare constructors. An abstract type MUST NOT: there is
  no single value of an abstract type to build, and calling one is refused with
  a message naming its concrete subtypes, as HQL-0016 already requires.
- A record type's constructor takes its fields as named arguments,
  `Edge(source = a, target = b, data = {})`, once declared records can be
  constructed ([issue 0016](../../issues/0016-sql-parity-analysis.md),
  roadblock "record and pair values").
- A format type of [HQL-0017](../HQL-0017/README.md) has one constructor from
  `String`, which checks that the text parses in its format:
  `Json(s) : Json`. It is not `format`: `Json(s)` reads `s` as JSON source,
  while `format(s, Json)` writes the string `s` as a JSON value.

### 2. Declaring a constructor

A constructor is a function named after its type:

```hql
fn Set<T>(xs: Collection<T>) : Set<T>
fn Graph(xs: Collection<Card>) : HmdGraph
fn Json(s: String) : Json
```

- A function MAY be named in `UpperCamelCase` only if it is a constructor: its
  name is a type declared in the same module, and it returns that type or one
  of its subtypes.
- All constructors of a type MUST be declared in the module that declares the
  type. Another module builds values of the type through them, or declares its
  own subtype with its own constructors, as the knowledge extension does with
  `KnowledgeGraph`.
- A type MAY have several constructors. They MUST differ in the type of their
  first parameter, so that the first argument alone selects one, as overloads
  of one operation do.

### 3. Two forms of one call

```text
T(x)          ≡  x | T
T(x, a, …)    ≡  x | T(a, …)
T<A>(x, …)    ≡  x | T<A>(…)
```

- Both forms MUST check and evaluate identically.
- The first parameter MAY be of any type. `Set(xs)` takes a collection,
  `Graph(edges)` a collection of edges, `Json(s)` a string; the pipeline form
  carries whatever the first parameter takes.
- In pipeline position, a bare type name with no arguments MUST resolve to its
  constructor. `<` after it opens type arguments under the rule the
  expressions model already gives: `Name<…>(` is a constructor only when the
  angle brackets hold type names alone.
- A constructor MUST be a pure operation. It builds a new value and does not
  read a source or the host.

### 4. A constructor as a function value

A constructor is a function value with its signature
([issue 0009](../../issues/0009-function-type-and-constructor.md)):

```hql
groups | map(Set)        // each List<T> to a Set<T>
texts  | map(Json)       // each String checked as JSON
```

- A type with several constructors, passed as a value, MUST be resolved by the
  parameter type the receiving operation expects, and a call that leaves it
  ambiguous MUST be refused, naming the candidate first-parameter types.

### 5. No second name for a constructor

- An operation whose result is a value of type `T` built from its input, and
  which does nothing else, MUST NOT be declared beside `T`'s constructor. It is
  a second name for the same signature and result.
- `list` ([HQL-0016](../HQL-0016/README.md)) and `table`
  ([HQL-0017](../HQL-0017/README.md)) MUST be unknown operations, each with a
  diagnostic naming its constructor.
- `graph` is the binding `graph = vault.graph`
  ([HQL-0019](../HQL-0019/README.md)), a value. Written as a step, it MUST be
  refused with a diagnostic naming the constructor: `` `graph` is the vault's
  knowledge graph, not an operation; `Graph` constructs a graph ``.
- An unknown lowercase name that matches a type's name apart from case SHOULD
  get a diagnostic naming the constructor.
- A lowercase operation that does more than construct is not a second name and
  stays: `sort` orders a collection into a `List`, and `markdown` renders any
  value as `Markdown`.

## Backwards Compatibility

Breaking where a lowercase constructor alias exists. `table` becomes an
unknown operation and `graph` a value; `list` is retired by HQL-0016. The collection and
map constructors keep their call form and gain the pipeline form. `UpperCamelCase` function names were not accepted
before, so restricting them to constructors breaks no program.

## Reference Implementation

- `src/constructors.rs` — constructor lookup by type name and first-argument
  type; the pipeline form
- `src/checker.rs` — a bare type name in step position resolving to its
  constructor; the refusal of an abstract type; the `UpperCamelCase` rule for
  `fn`
- `src/extensions/mod.rs` — the case-insensitive suggestion for an unknown
  operation
- `std/*.hql` — each constructor declared beside its type
- `doc/models/behavior/expressions.md` — the two forms, replacing the open
  sentence about a constructor in other positions
- `doc/wiki/hql/type-system.hmd` — constructors as the way a value of a type is
  built

## Test Plan

Unit tests MUST include:

- `xs | Set` equal to `Set(xs)`; `keys | Map(values)` equal to
  `Map(keys, values)`; `[] | List<Int>` equal to `List<Int>([])`
- `cards | Graph` equal to `Graph(cards)`
- `xs | Seq` refused, naming `List`
- `fn Total(xs: List<Int>) : Int` refused: `Total` names no type
- `table` and `list` refused, each naming its constructor; `cards | graph`
  refused naming `Graph`
- `groups | map(Set)` checks; a type with two
  constructors passed where the expected parameter type does not pick one is
  refused, naming both

```bash
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## Changelog

- 2026-10-18: drafted
- 2026-10-19: accepted; the pipeline form of constructors, the refusal of an
  abstract type, constructors as function values (`map(Set)`) and
  `UpperCamelCase` reserved for constructors are implemented, and constructors
  declared in `std/`, overloads by first argument, record constructors and
  `Json(s)` are not yet
