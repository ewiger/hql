# `lib/std/` — the HQL standard library

The type declarations HQL ships with, written in HQL.

**Status: parsed and checked.** `hql check lib/std/graph/mod.hql` prints `Unit`, and
`tests/declarations.rs` walks this folder, so a file added here cannot be
silently left unchecked. These files are the single place the declarations live
as *source* rather than as prose inside a card. The modules written as external
declarations, `string`, `seq`, `math`, `option`, `result` and `regex`, are the
contracts the package's internal extension, `std`, implements, checked against
it by name;
`io` and `io.logging` declare their operations beside their types.
`validators` is the one module written in HQL, its implementations bodies a
program runs: it is loaded when a program writes `import validators`, as a
workspace module is, and declares the standard validator types
([HQL-0034](../../doc/proposals/HQL-0034/README.md)).

Checking a declaration is not evaluating one. A declaration yields `Unit` and
introduces no value; constructing a value of a declared record type is separate
work. What is checked is that each declaration is coherent, and that where it
restates a type the binary already holds it agrees with it — see
[the grammar](../../doc/models/behavior/expressions.md).

`type Data = union { Scalar, Seq<Data>, Map<String, Data> }` is the one union
here: a tree is one of its members rather than all of its parents. The binary
holds the same three members, and restating it with one more or fewer is
refused.

`abstract type` declares a contract without a runtime constructor. Collection
declarations and their `where` bounds agree with the Rust `TypeConstructor` /
`TypeRef` registry. `List` and `Set` materialize occurrences; `Map`, `OrderedMap`,
and `SortedMap` materialize associations. `SortedMap` requires orderable keys.

A type's constructors are declared as functions named after it, beside it:
`fn Set<T>(xs: Collection<T>) : Set<T>` and `fn Data` in `core`, `fn Graph` in
`graph`, `fn Channel` in `io`. Binding
holds each to the binary's constructor, as it holds an external operation to
its extension
([HQL-0018](../../doc/proposals/HQL-0018/README.md)).

Collection operations are core functions and also pipeline steps. For example,
`count(["owl", "owl"], "owl")` returns `2`, and `sort([2, 1])` returns a
`List<Int>`. The signatures beside the declarations document these implemented
functions; they are not user-defined function bodies.

See the runnable [birds wiki](../../examples/birds/README.md) for collection
literals, construction, key projections, lookup, and explicit comparison.

## What belongs here, and what does not

| Place | Holds |
| --- | --- |
| `lib/std/` | the declarations themselves, as HQL |
| `doc/wiki/hql/` | cards arguing *why* a declaration is what it is |
| `doc/models/` | the domains the declarations describe |
| `examples/` | design inputs, explicitly allowed to keep superseded alternatives |
| `tests/fixtures/` | inputs to tests |

The distinction that matters is the fourth row. The corpus under `examples/` may
hold a withdrawn spelling beside a current one, because its job is to record
what was considered. A standard library may not: there is one declaration of
`Edge`, and it is the one that is true now. Keeping them apart is what stops a
reader finding two answers and having to date them.

A card and a file here will say the same thing twice, which is a real cost.
The rule is that the file is the declaration and the card is the argument: when
they disagree, the card explains something that no longer exists, and the card
is what gets fixed.

## The package and its modules

`lib/` holds the builtin packages, one folder each: their contracts are
bundled with the binary and the extension of each, one per package and named
as it is, compiled into it: an internal extension. `lib/std/` is
the standard package, the one builtin package whose prelude is the implicit
import list. Its root is a module root, not a module, and holds two kinds of
thing:

- `prelude.hql`, the package's global space: not a module, it runs when the
  package loads, and its imports are the implicit import list of every source
  unit outside the package
  ([HQL-0024 part 3](../../doc/proposals/HQL-0024/parts/03-standard-library.md)).
- One folder per module, the module's declarations in the folder's `mod.hql`:
  `core/mod.hql` is `core`, `io/net/mod.hql` is `io.net`. No declaration sits
  outside a module.

Every module is its own import unit
([HQL-0028](../../doc/proposals/HQL-0028/README.md)). The package's prelude does
not apply to its own modules, so each `mod.hql` imports every module whose
types or traits it writes, `import core` included, and sees nothing else.
Types sit low and operations above them: a type that two modules both need
lives in a module both import, never in either. That is why the collection
types are `core`'s (`Data` is a tree of sequences and maps), why `Card` is
`doc`'s (a graph's nodes are cards, and knowledge narrows the graph), and why
the category `Format` is `core`'s (`Data(s, Json)` and every `io` operation
are bounded by it).

Binding refuses the whole environment when a module writes a name it neither
declares nor imports, imports a module listed after it or its own child, or
when two modules declare one name (`library::closure`). The modules are listed
in import order, so the import graph is acyclic by construction:

| Module | Imports | Holds | Implemented by |
| --- | --- | --- | --- |
| `core` | — | scalars, the collection types and their constructors, `Data`, `Option`, `Result`, `Ordering`, `Type`, `Is`, `Format`, `Range`, `Pair`, the operation types, the contract types and violations, the core traits `Eq`, `Ord`, `Add`, `Sub`, `Mul`, `Div`, `Rem`, `Index`, `Slice`, `Validator` and the built-in types' implementations of them | the `std` extension |
| `doc` | `core` | documents, their headers, and the card family `Card`, `HmdCard`, `MdCard` | the runtime |
| `retrieval` | `core` | `Hit` and `Ranking`, from [HQL-0002](../../doc/proposals/HQL-0002/README.md) | the runtime |
| `graph` | `core`, `doc`, `retrieval` | edges, graphs, links and the `Graph` constructor | the `std` extension |
| `knowledge` | `core`, `doc`, `graph` | concepts, relations, `ConceptCard`, `RelationCard`, `KnowledgeGraph` | the runtime |
| `collections` | `core`, `doc` | the operations over the collection types: membership, filtering, mapping, order, grouping, joins, reductions; `Group` | the `std` extension |
| `string` | `core` | `module string for String`: the text functions, `str` and `fmt` | the `std` extension |
| `seq` | `core` | `module seq for Seq<T>`: the sequence, set and map operations beyond the core's | the `std` extension |
| `math` | `core` | numbers and the conversions between `Int`, `Float` and `Decimal` | the `std` extension |
| `option` | `core` | `module option for Option<T>`: the ways out of an option | the `std` extension |
| `result` | `core` | `module result for Result<T, E>`: the ways out of a result | the `std` extension |
| `regex` | `core` | text matched against a pattern; imported where it is used | the `std` extension |
| `format` | `core`, `graph` | the formats and the codec contracts ([HQL-0024 part 7](../../doc/proposals/HQL-0024/parts/07-formats-and-codecs.md)) | the `std` extension |
| `io` | `core` | endpoints, channels, data sources and data targets, from [HQL-0021](../../doc/proposals/HQL-0021/README.md) | the `std` extension |
| `io.net` | `core`, `io` | network endpoints; `import io.net` or `import net` | the `std` extension |
| `io.logging` | `core`, `io` | loggers, levels, log events and the rendering cell; `import io.logging` or `import logging` | the `std` extension |
| `vault` | `core`, `doc`, `graph` | the vault a program runs against | the runtime |
| `validators` | `core`, `string`, `seq`, `option` | the standard validator types, written in HQL; `import validators` | the runtime |

A module the runtime implements declares nothing external, so no extension
provides it; importing it binds no operation.

A module written as **external declarations**, `fn trim(self) : String` with
no body, is a contract an extension implements: the extension whose descriptor
provides the module, here always `std`. Binding, once per process, holds the two to each other by
name, arity and result type, and refuses every program on a mismatch; the
wiring is on the [extensions card](../../doc/wiki/hql/extensions.hmd). A module
that opens with `module <name> for T` is a receiver module, whose `self` functions
resolve as `value.f()`; a declaration outside one writes its receiver's type,
`fn abs(self: Int) : Int`, and one without `self` is never reached as
`value.f()`. `effectful fn source(…)` declares an operation effectful, and a
plain `fn` pure; `elementwise fn`, `partition fn` and `reduction fn` declare
how an executor may split its input, and a plain `fn` is a barrier. Binding
holds the extension's own markings to both.

## Conventions

Generics are written with **angle brackets** in both positions — `Set<T>`,
`Edge<S, T>` — and square brackets stay value-level. See
[bind versus apply](../../doc/models/behavior/bind-vs-apply-in-generic-types.md).

Every declaration here appears in a card. Nothing is invented in this folder: if
a declaration is not settled, it stays in the card as an open question until it
is, rather than being written here to look decided.
