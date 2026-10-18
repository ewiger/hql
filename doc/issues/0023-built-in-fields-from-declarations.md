# 0023 — Built-in record fields from their declarations

Status: done
Source: the `Ranking` feature of [0014](0014-next-feature-bundle.md), which
made the retrieval carrier generic and left this part

The fields of `Hit`, `Ranking`, `Doc`, `Card`, `Edge` and `Graph` are a
hand-written table in `checker::field_type`, and the declarations in `std/`
disagree with it. The table is what programs use today, so it is the
behaviour; the declarations are what the cards argue. Reading fields from the
declarations is one change for all of them, once each disagreement is
decided.

## Resolution

`std/` is embedded in the binary (`src/library.rs`), checked once as one
program, and a field on a built-in record is read from its declaration, then
from its parents, with the receiver's type arguments bound. Where `std/` and
the checker disagreed, `std/` now states what programs already read:

| Type | Fields |
| --- | --- |
| `Hit<T>` | `value : T`, `score : Float`, `query : String`, `retrieval : Data` |
| `Ranking<T> : Seq<Hit<T>>` | `hits : List<Hit<T>>`, `query : String`, `retrieval : Data` |
| `Doc` | `name`, `title`, `path`, `format : String`, `header : Data`, `body : String` |
| `Card` | `metadata : Data`, `kind : String` |
| `Edge<S, T>` | `source : String`, `target : String`, `data : Data` |
| `Graph<N, E>` | `nodes : List<String>`, `edges : List<Edge>` |

`checker::field_type` keeps three arms that are not records: `Option` lifts a
field read, `Data` is an open tree, and a map's `keys` depends on the map's
kind. `tests/declarations.rs` checks that every module of `std/` is embedded as
written, checks after the modules it follows, and gives every built-in field
the type its declaration states.

## Conflict

[HQL-0019](../proposals/HQL-0019/README.md) specifies `g.nodes : Set<N>` and
`g.edges : Set<E>`. The declared and implemented fields are
`List<String>` and `List<Edge>`, because the graph carriers are not generic;
HQL-0019's fields arrive with generic carriers.
