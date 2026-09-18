# 0004 — Unify the collection-order vocabulary

Status: done
Branch: `feat/collection-order`

Four names touched order, and it was not obvious that four were needed:

| Name | What it says | Whose property |
| --- | --- | --- |
| `Set<T>` | membership only; no first element | the collection |
| `Seq<T>` | a position exists for every element | the collection |
| `List<T> <: Seq<T>` | the elements are held, not produced | the collection |
| `Orderable` | the value carries an ordering key of its own | the **element** |

Three are needed. `List` is withdrawn, and
[collections](../wiki/hql/collections.hmd) now owns the whole account of order.

## Does `List` earn its place — no

`Seq<T>` is an **abstract type governed by laws**, and `A <: B` asserts that `A`
satisfies the laws of `B` — see [type system](../wiki/hql/type-system.hmd),
*Abstract types and their laws*. Read that way, `Seq` versus `List` is not a
naming competition. `Seq<T>` is the mathematical abstraction HQL cares about;
`List<T>` is a claim about **representation**, that the elements are held rather
than produced. It earns its place only if the language ever needs to tell two
representations of the same abstraction apart.

Nothing does. The bootstrap evaluator's `Value::Seq` holds an `Rc<Vec<Value>>`,
the only representation implemented is the materialized one, and the checker
resolved `List` to `Seq` from the day it was written — so the binary never
distinguished them either. `List` is withdrawn from `std/collections.hql` and
from `types::named`, and `Value::Seq` keeps its name honestly because there is
one representation to have.

If HQL later finds the case — a streamed ranking, a bounded result set — the
name returns with the case that justifies it.

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

What that left, and how each one went:

- **`Ordering` and `SortedSeq<T>` were named and declared nowhere.** Both are
  dropped from the prose rather than declared. `SortedSeq<T>` fails the same
  test `List` fails: nothing downstream needs to know a sequence is sorted, and
  the one shape that has the property already has a name — `Ranking<T>`. And the
  `sort` that takes its order from the caller takes a **key extractor**, which
  is what `sort(by = c => c.title)` already is, so no comparator and no
  `Ordering` result type is needed.
- **`take(n)` over a `Set` needs `Orderable`**, which is what is implemented: the
  prefix comes from the elements' own keys. "There is a canonical `Seq` for this
  `Set`" is the same statement made from the collection's side, and saying it
  twice is what this issue was for.
- **`Ranking<T> <: Seq<Hit<T>>` and `Hit <: Orderable` are both true, for
  different reasons** — positional order belonging to the collection, value order
  to the element — and they *agree*, which is the third concept and the only
  place the language has it.
  [Orderable](../wiki/hql/types/orderable.hmd) now says so under *A ranking
  carries both orders*.
- **`Orderable` is one total order per type.** Several named keys would leave
  `take(5)` over a `Set` with no canonical prefix, which is the vault order the
  card exists to keep unobservable. A second ordering arrives at the call site
  as `sort(by = …)`, so nothing is lost.

## Scope

Documentation and `std/`, plus the one line of `src/` the decision reaches:
`types::named` resolved `List` to `Seq`, and leaving it there would have kept a
name the library no longer declares.

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
