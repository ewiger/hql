# 0004 — Unify the collection-order vocabulary

Status: backlog
Branch: `feat/collection-order`

Four names now touch order, and it is not obvious that four are needed:

| Name | What it says | Whose property |
| --- | --- | --- |
| `Set<T>` | membership only; no first element | the collection |
| `Seq<T>` | a position exists for every element | the collection |
| `List<T> : Seq<T>` | the elements are held, not produced | the collection |
| `Orderable` | the value carries an ordering key of its own | the **element** |

`List<T> : Seq<T>` is declared in [`std/collections.hql`](../../std/collections.hql) and
argued in [collections](../wiki/hql/collections.hmd). What is not settled is
whether the vocabulary needs all four names.


> TODO: this card needs compelte rewrite as soon as the type systems will be fully defined and mapped to propoer rust implementation.