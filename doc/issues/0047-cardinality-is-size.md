# 0047 — Cardinality is `size`: a String's `size` replaces `length`

Status: todo
Related: [HQL-0014](../proposals/HQL-0014/README.md),
[HQL-0026](../proposals/HQL-0026/README.md),
[`lib/std/string`](../../lib/std/string/mod.hql)

Cardinality has one name, `size`, with `count(xs)` its one kept alias
(HQL-0014, "Named aggregations are aliases"). A String declares its own
`size(self) : Int`, its number of code points, and `length` is withdrawn.
A String's positions are its code points, counted by `s.size()` (HQL-0026,
"What each receiver gives").

## Work

`lib/std/string/mod.hql` declares `fn size(self) : Int` beside `fn length(self) :
Int`, its doc string calling it the same as `length`; what remains is
withdrawing `length`.

1. `lib/std/string/mod.hql`: `fn length(self) : Int` removed, and `size`'s doc
   string stating the law `s.size() == s.chars().size()`.
2. The `string` extension: the operation renamed, its description naming code
   points.
3. `s.length()` refused with a message naming `s.size()`.
4. Tests: a String's `size` over multi-byte text, beside `chars` and
   `bytes`; the refusal of `length`.
5. Cards and models that name `length`: the bracket operator card, the
   operation-signatures model, and every example and card under `doc/` and
   `examples/` that calls it.
6. HQL-0014 and HQL-0026 changelogs: mark the decision implemented.
