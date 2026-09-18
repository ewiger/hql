# `std/` — the HQL standard library

The type declarations HQL ships with, written in HQL.

**Status: parsed and checked.** `hql check std/graph.hql` prints `Unit`, and
`tests/declarations.rs` walks this folder, so a file added here cannot be
silently left unchecked. These files are the single place the declarations live
as *source* rather than as prose inside a card.

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

One file per module, mirroring the extension split in
[HQL-0001](../doc/proposals/HQL-0001/README.md), so a module's declarations sit
where its steps will:

| File | Module |
| --- | --- |
| `core.hql` | the scalar, data, absence and value-order vocabulary the core owns |
| `collections.hql` | membership, order and lookup |
| `doc.hql` | documents |
| `graph.hql` | the graph domain |
| `knowledge.hql` | the knowledge domain, including the card family |
| `retrieval.hql` | ranking, arriving with [HQL-0002](../doc/proposals/HQL-0002/README.md) |

## Conventions

Generics are written with **angle brackets** in both positions — `Set<T>`,
`Edge<S, T>` — and square brackets stay value-level. See
[bind versus apply](../doc/models/behavior/bind-vs-apply-in-generic-types.md).

Every declaration here appears in a card. Nothing is invented in this folder: if
a declaration is not settled, it stays in the card as an open question until it
is, rather than being written here to look decided.
