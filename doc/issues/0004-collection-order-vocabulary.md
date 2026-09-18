# 0004 — Unify the collection-order vocabulary

Status: backlog
Branch: `feat/collection-order`

Four names now touch order, and it is not obvious that four are needed:

| Name | What it says | Whose property |
| --- | --- | --- |
| `Set<T>` | membership only; no first element | the collection |
| `Seq<T>` | a position exists for every element | the collection |
| `List<T> <: Seq<T>` | the elements are held, not produced | the collection |
| `Orderable` | the value carries an ordering key of its own | the **element** |

`List<T> <: Seq<T>` is declared in [`std/collections.hql`](../../std/collections.hql) and
argued in [collections](../wiki/hql/collections.hmd). What is not settled is
whether the vocabulary needs all four names.

## Does `List` earn its place

`Seq<T>` is an **abstract type governed by laws**, and `A <: B` asserts that `A`
satisfies the laws of `B` — see [type system](../wiki/hql/type-system.hmd),
*Abstract types and their laws*. Read that way, `Seq` versus `List` is not a
naming competition. `Seq<T>` is the mathematical abstraction HQL cares about;
`List<T>` is a claim about **representation**, that the elements are held rather
than produced. It earns its place only if the language ever needs to tell two
representations of the same abstraction apart.

Nothing currently does. The bootstrap evaluator's `Value::Seq` holds an
`Rc<Vec<Value>>` — the only representation implemented is the materialized one,
so the evaluator's `Seq` *is* a `List`. Either:

- HQL finds a case that needs the distinction — a lazily produced sequence, a
  streamed ranking, a bounded result set — and `List` stays; or
- it does not, `List` is withdrawn from `std/collections.hql`, and `Value::Seq` keeps
  its name honestly because there is only one representation to have.

Settling this also decides whether `Value::Seq` should be renamed `Value::List`.

## The question

**Settled.** `Seq` and `Orderable` are two different claims, not one seen from
either end. `Seq<T>` is positional order, decided by the collection;
`Orderable` is value order, carried by the element type. `Seq<Function>` proves
they are independent — it has a first and a second element while no two
functions have a meaningful `<`. Sorting is the bridge, and it is a **signature,
not a supertype**: `fn sort<T>(xs: Seq<T>) : Seq<T> where T <: Orderable`, or
`fn sort<T>(xs: Seq<T>, by: (T, T) -> Ordering) : Seq<T>` when the caller
supplies the relation instead. Written up in
[collections](../wiki/hql/collections.hmd), *Ordered is not orderable*.

What that leaves:

- **`Ordering` and `SortedSeq<T>` are named and declared nowhere.** Neither is in
  [core](../wiki/hql/core.hmd) or `std/collections.hql`. `SortedSeq<T>` is the type
  whose positional order agrees with its value order, and it is the honest result
  type of the first `sort` above.
- Does `take(n)` over a `Set` need `Orderable`, or does it need "there is a
  canonical `Seq` for this `Set`", which is a statement about the collection?
- Is `Ranking<T> <: Seq<Hit<T>>` carrying order, or is `Hit <: Orderable`
  carrying it? [Orderable](../wiki/hql/types/orderable.hmd) currently says both.
  Under the settled distinction it should be both, for different reasons — but
  the card should say so rather than leave it read as one fact stated twice.
- Is `Orderable` one total order per type, or several named keys? Sorting expects
  a total order, so this decides what the first `sort` signature actually
  guarantees.

## Scope

Documentation and `std/` only. Nothing here is implementable until
[issue 0003](0003-type-declarations.md) makes `std/` parse, and `Orderable` is
already implemented in `src/` under the current reading.

## Done when

- One card owns the account of order, and the others link to it instead of
  restating it.
- `List<T>` is either justified by a case that needs it or withdrawn, and
  `Value::Seq` is named after whichever answer wins.
- `Ordering` and `SortedSeq<T>` are either declared in `std/collections.hql` and
  [core](../wiki/hql/core.hmd), or dropped from the prose that names them.
- `std/collections.hql` matches whatever that decision is.
- The Orderable card's "single total order or several named keys" question is
  answered, or moved to a proposal if it is larger than this issue.
