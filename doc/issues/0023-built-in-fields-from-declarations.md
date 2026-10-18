# 0023 — Built-in record fields from their declarations

Status: backlog
Source: the `Ranking` feature of [0014](0014-next-feature-bundle.md), which
made the retrieval carrier generic and left this part

The fields of `Hit`, `Ranking`, `Doc`, `Card`, `Edge` and `Graph` are a
hand-written table in `checker::field_type`, and the declarations in `std/`
disagree with it. The table is what programs use today, so it is the
behaviour; the declarations are what the cards argue. Reading fields from the
declarations is one change for all of them, once each disagreement is
decided.

## The disagreements

| Type | `std/` declares | The checker gives |
| --- | --- | --- |
| `Hit<T>` | `value : T`, `score : Score`, `provenance : Retrieval` | `value : T`, `score : Float`, `query : String`, `retrieval : Data` |
| `Ranking<T>` | no fields | `hits : List<Hit<T>>`, `query : String`, `retrieval : Data` |
| `Doc` | `name`, `path`, `format`, `header : Data`, `body : Content` | the same with `body : String`, and `title : String` |
| `Card` | `metadata : Data` | `metadata : Data`, `kind : String` |
| `Graph<N, E>` | no fields | `nodes : List<String>`, `edges : List<Edge>`; [HQL-0019](../proposals/HQL-0019/README.md) wants `Set<N>` and `Set<E>` |

## Work

1. Decide each row: `Score`, `Query` and `Retrieval` as declared types or as
   `Float`, `String` and a record; `title` and `kind` declared or retired;
   `Graph`'s fields per HQL-0019.
2. Embed `std/` in the binary and read record fields from the parsed
   declarations, so `field_type` holds no per-type table.
3. Keep `tests/declarations.rs` checking that every declaration agrees with
   the built-in lattice.

## Done when

- `checker::field_type` has no arm naming a built-in record type.
- Every field a program reads on a built-in record is declared in `std/`.
- `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`
  and `cargo fmt --check` are clean.
