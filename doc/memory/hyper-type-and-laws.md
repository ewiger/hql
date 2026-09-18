# Remaining retrieval type decisions

Collection declarations, checking, and runtime values now share `TypeRef` and
`TypeConstructor`. Their current contracts live in `std/collections.hql`, the
collection cards, and `doc/models/behavior/expressions.md`; the earlier marker
and law-witness arrangement has been replaced.

`Hit` and `Ranking` still have generic type references and monomorphic carriers:
`Value::type_of` reports `Hit<Card>` for a hit. Supporting a narrower retrieved
element requires preserving that type in the carrier.

Removing the `Ranking` constructor is still wanted. The cost to weigh first:
it gives `ranking.hits`, `ranking.query`, and `ranking.retrieval` a type in
`checker::field_type`; representing a ranking as `Seq<Hit<Card>>` loses those
fields. Each hit carries its own provenance, so the data survives, while the
top-level field access does not.
