# Remaining retrieval type decisions

The `Ranking` carrier is generic: a hit holds any value at the element type its
ranking was built from, and `h.value` reads it. One part of making built-in
records ordinary types remains, and it is a decision, not a refactoring:

- **Built-in fields are a hand-written table** in `checker::field_type`, and
  the declarations in `std/` disagree with it: `Hit` declares `score : Score`
  and `provenance : Retrieval` where the checker gives `score : Float`,
  `query : String` and `retrieval : Data`; `Doc` declares `body : Content`
  where the checker gives `String`, and no `title` or `kind`; `Graph` declares
  no fields where the checker gives `nodes` and `edges`. Reading fields from
  declarations means choosing, field by field, which side is right.

Home when digested: issue [0023](../issues/0023-built-in-fields-from-declarations.md).
