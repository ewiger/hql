# 0041 — Joins over one other sequence: HQL-0013's binary form

Status: done
Related: [HQL-0013](../proposals/HQL-0013/README.md),
[HQL-0012](../proposals/HQL-0012/README.md),
[the data sources examples](../../examples/sources/README.md)

## Scope

- `join`, `left_join`, `right_join`, `full_join`, `cross_join`, `semi_join`
  and `anti_join` over the pipeline value and one other sequence, declared
  in `lib/std/collections/mod.hql` and implemented in `src/extensions/collections.rs`.
- The refusals of the section "Conditions": no condition names
  `cross_join`, `left` needs `right`; a `Set` input names `sort`.
- Module resolution: where several visible modules declare a name, the
  names of the call's named arguments choose among them, and the arguments
  written by name count toward a form's required ones.

## Done when

- `tests/joins.rs` checks each mode over duplicates on both sides in the
  order of positions; keys, `on` and both; `semi_join` keeping an element
  once; the refusals; and that `string.join` and `seq.join` still resolve.
- `examples/sources/queries/journal-counts.hql` joins a vault source to a
  CSV source by species.

## Left open

- The head and extending forms, row types `{b: Card, s: Data}`, optional
  sides `?`, and joins over streams.
