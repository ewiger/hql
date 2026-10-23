# `std/` — the HQL standard library

The type declarations HQL ships with, written in HQL.

**Status: parsed and checked.** `hql check std/graph.hql` prints `Unit`, and
`tests/declarations.rs` walks this folder, so a file added here cannot be
silently left unchecked. These files are the single place the declarations live
as *source* rather than as prose inside a card. The modules written as external
declarations, `string`, `seq`, `math`, `option`, `result` and `regex`, are the
contracts the binary's extensions implement, checked against them by name;
`io` and `io.logging` declare their operations beside their types.
`validators` is the one module written in HQL, its implementations bodies a
program runs: it is loaded when a program writes `import validators`, as a
workspace module is, and declares the standard validator types
([HQL-0034](../doc/proposals/HQL-0034/README.md)).

Checking a declaration is not evaluating one. A declaration yields `Unit` and
introduces no value; constructing a value of a declared record type is separate
work. What is checked is that each declaration is coherent, and that where it
restates a type the binary already holds it agrees with it — see
[the grammar](../doc/models/behavior/expressions.md).

`type Data = union { Scalar, Seq<Data>, Map<String, Data> }` is the one union
here: a tree is one of its members rather than all of its parents. The binary
holds the same three members, and restating it with one more or fewer is
refused.

`abstract type` declares a contract without a runtime constructor. Collection
declarations and their `where` bounds agree with the Rust `TypeConstructor` /
`TypeRef` registry. `List` and `Set` materialize occurrences; `Map`, `OrderedMap`,
and `SortedMap` materialize associations. `SortedMap` requires orderable keys.

A type's constructors are declared as functions named after it, beside it:
`fn Set<T>(xs: Collection<T>) : Set<T>` in `collections.hql`, `fn Graph` in
`graph.hql`, `fn Channel` in `io/mod.hql`, `fn Data` in `core.hql`. Binding
holds each to the binary's constructor, as it holds an external operation to
its extension
([HQL-0018](../doc/proposals/HQL-0018/README.md)).

Collection operations are core functions and also pipeline steps. For example,
`count(["owl", "owl"], "owl")` returns `2`, and `sort([2, 1])` returns a
`List<Int>`. The signatures beside the declarations document these implemented
functions; they are not user-defined function bodies.

See the runnable [birds wiki](../examples/birds/README.md) for collection
literals, construction, key projections, lookup, and explicit comparison.

## What belongs here, and what does not

| Place | Holds |
| --- | --- |
| `std/` | the declarations themselves, as HQL |
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

## Layout

One file per module, a folder for a module with submodules (`io/mod.hql` is
`io`, `io/net.hql` is `io.net`), mirroring the extension split in
[HQL-0001](../doc/proposals/HQL-0001/README.md), so a module's declarations sit
where its steps will:

| File | Module |
| --- | --- |
| `prelude.hql` | not a module: the implicit import list every source unit starts with ([HQL-0024 part 3](../doc/proposals/HQL-0024/parts/03-standard-library.md)) |
| `core.hql` | the scalar, data, absence, result, pair and value-order vocabulary the core owns, and the core traits `Eq`, `Ord`, `Add`, `Sub`, `Mul`, `Div`, `Rem` |
| `collections.hql` | membership, order and lookup |
| `string.hql` | `module for String`: the text functions, `str` and `fmt`; external, implemented by the `string` extension |
| `seq.hql` | `module for Seq<T>`: the sequence, set and map operations beyond the core's; external |
| `math.hql` | numbers and the conversions between `Int`, `Float` and `Decimal`; external |
| `option.hql` | `module for Option<T>`: the ways out of an option; external |
| `result.hql` | `module for Result<T, E>`: the ways out of a result; external |
| `regex.hql` | text matched against a pattern; external, imported where it is used |
| `doc.hql` | documents |
| `format.hql` | the formats, a category of their own, and the codec contracts ([HQL-0024 part 7](../doc/proposals/HQL-0024/parts/07-formats-and-codecs.md)) |
| `graph.hql` | the graph domain |
| `io/mod.hql` | endpoints, channels, data sources and data targets, from [HQL-0021](../doc/proposals/HQL-0021/README.md); `file`, `source`, `tap`, `target` and `sink` external |
| `io/net.hql` | network endpoints, arriving with HQL-0021; `import io.net` or `import net` |
| `io/logging.hql` | loggers and log events, from HQL-0021; `Logger` and the five levels external; `import io.logging` or `import logging` |
| `knowledge.hql` | the knowledge domain, including the card family |
| `retrieval.hql` | ranking, arriving with [HQL-0002](../doc/proposals/HQL-0002/README.md) |

A module written as **external declarations**, `fn trim(self) : String` with
no body, is a contract an extension implements: the extension whose descriptor
provides the module. Binding, once per process, holds the two to each other by
name, arity and result type, and refuses every program on a mismatch; the
wiring is on the [extensions card](../doc/wiki/hql/extensions.hmd). A module
that opens with `module for T` is a receiver module, whose `self` functions
resolve as `value.f()`; a declaration outside one writes its receiver's type,
`fn abs(self: Int) : Int`, and one without `self` is never reached as
`value.f()`. `effectful fn source(…)` declares an operation effectful, and a
plain `fn` pure; `elementwise fn`, `partition fn` and `reduction fn` declare
how an executor may split its input, and a plain `fn` is a barrier. Binding
holds the extension's own markings to both.

## Conventions

Generics are written with **angle brackets** in both positions — `Set<T>`,
`Edge<S, T>` — and square brackets stay value-level. See
[bind versus apply](../doc/models/behavior/bind-vs-apply-in-generic-types.md).

Every declaration here appears in a card. Nothing is invented in this folder: if
a declaration is not settled, it stays in the card as an open question until it
is, rather than being written here to look decided.
