# 0027 — A partial record type reads card metadata as typed fields

Status: todo
Depends on: declared record types in the type lattice, the type-system work
that follows [HQL-0016](../proposals/HQL-0016/README.md)
Source: the [todo example](../../examples/todo/README.md)

`Card.metadata` is `Data`, so a metadata read is `Option<Data>`, and `Data` is
not [`Orderable`](../wiki/hql/types/orderable.hmd). A query that orders cards by
a metadata key is refused:

```hql
cards
| filter(c => c.metadata.status == "todo")
| sort(by = c => c.metadata.priority)
| map(c => c.title)
```

```
type error: a sort key must return Orderable, not Option<Data>
```

The todo example works around it with a typed `match` per key,
`match c.metadata.priority { some(Int p) => p, _ => 0 }`, which restates the
key's type at every use and supplies a default the data does not have.

A declared type cannot replace the `match` today: a `type` statement is checked
for coherence and enters no lattice ([`src/declarations.rs`](../../src/declarations.rs)),
so `of_type(TaskCard)` is a name error.

## Scope

- A program declares a record type for the part of the metadata it reads, and a
  card subtype whose `metadata` field has that type. The record is partial:
  keys it does not name stay in the tree and are not checked.

  ```hql
  type Task {
      status   : String
      priority : Int
  }

  type TaskCard : Card {
      metadata : Task
  }

  cards
  | of_type(TaskCard)
  | filter(c => c.metadata.status == "todo")
  | sort(by = c => c.metadata.priority)
  | map(c => c.title)
  ```

- `of_type(TaskCard)` keeps a card exactly when every field `Task` declares is
  present in its metadata and its leaf reads as the declared type, through
  `Value::interpreted`. A card that does not fit is dropped, not reported:
  `of_type` is a filter ([HQL-0016](../proposals/HQL-0016/README.md)).
- On a kept card, `c.metadata` is a `Task`, and `c.metadata.priority` is an
  `Int` rather than an `Option<Data>`, so it is a sort key. An optional field,
  `due? : String`, reads as `Option<String>`.
- `Data` stays not `Orderable`; the order comes from the declared field type,
  not from the tree.

## Done when

- [`examples/todo/queries/todo-list.hql`](../../examples/todo/queries/todo-list.hql)
  and [`adr-overview.hql`](../../examples/todo/queries/adr-overview.hql) declare
  `TaskCard` and `AdrCard` (`date : String`) and sort by the field, with no
  `match`, and keep their expected results.
- A corpus case shows a card missing a declared key dropped by `of_type`.
- The [Card type](../wiki/hql/types/card-type.hmd) card states how a card
  subtype narrows `metadata`.
