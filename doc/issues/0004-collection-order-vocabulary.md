# 0004 — Unify the collection-order vocabulary

Status: done

Four names touch order. The question was whether all four are needed. They are,
because they answer two different questions:

| Name | What it says | Whose property |
| --- | --- | --- |
| `Set<T>` | membership only; no first element | the collection |
| `Seq<T>` | a position exists for every element | the collection |
| `List<T> : Seq<T>` | the elements are held, not produced | the collection |
| `Orderable` | the value carries an ordering key of its own | the **element** |

Position and order are independent: a `Seq<Bool>` has a first element and no
order, and an `Orderable` element in a `Set` has an order and no position. No
name can be folded into another.

## Resolution

- The element side is stated on the
  [orderable](../wiki/hql/types/orderable.hmd) card, which also fixes the red
  `[[orderable]]` links across the wiki. The collection side stays on
  [collections](../wiki/hql/collections.hmd).
- [`std/doc/mod.hql`](../../std/doc/mod.hql) declares `Doc : Orderable`, matching the
  built-in registry; every `Card` is orderable through `Doc`.
- The checker already enforces the split: bare `sort` and `take` on a `Set`
  require `Orderable` elements, and a sort key must return `Orderable`.
