# 0023 — Built-in record fields from their declarations

Status: done
Source: the `Ranking` feature of [0014](0014-next-feature-bundle.md), which
made the retrieval carrier generic and left this part

The fields of `Hit`, `Ranking`, `Doc`, `Card`, `Edge` and `Graph` are a
hand-written table in `checker::field_type`, and the declarations in `lib/std/`
disagree with it. The table is what programs use today, so it is the
behaviour; the declarations are what the cards argue. Reading fields from the
declarations is one change for all of them, once each disagreement is
decided.

## Resolution

`lib/std/` is embedded in the binary (`src/library.rs`), checked once as one
program, and a field on a built-in record is read from its declaration, then
from its parents, with the receiver's type arguments bound. Where `lib/std/` and
the checker disagreed, `lib/std/` now states what programs already read:

| Type | Fields |
| --- | --- |
| `Hit<T>` | `value : T`, `score : Float`, `query : String`, `retrieval : Data` |
| `Ranking<T> : Seq<Hit<T>>` | `hits : List<Hit<T>>`, `query : String`, `retrieval : Data` |
| `Doc` | `name`, `title`, `path`, `format : String`, `header : Data`, `body : String` |
| `Card` | `metadata : Data`, `kind : String` |
| `Edge<S, T>` | `source : String`, `target : String`, `data : Data` |
| `Graph<N, E>` | `nodes : Set<Card>`, `edges : Set<Edge>` |

`checker::field_type` keeps three arms that are not records: `Option` lifts a
field read, `Data` is an open tree, and a map's `keys` depends on the map's
kind. `tests/declarations.rs` checks that every module of `lib/std/` is embedded as
written, checks after the modules it follows, and gives every built-in field
the type its declaration states.

A graph's fields are sets over the vault's cards, `Set<Card>` and
`Set<Edge>`; [HQL-0019](../proposals/HQL-0019/README.md)'s `Set<N>` and
`Set<E>` arrive when the graph value carries its node and edge types.
