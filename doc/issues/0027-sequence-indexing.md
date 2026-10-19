# 0027 — Indexing a sequence by position

Status: done
Related: [core](../wiki/hql/core.hmd), [type system](../wiki/hql/type-system.hmd),
[issue 0015](0015-jq-parity-analysis.md), [HQL-0021](../proposals/HQL-0021/README.md)

The cards reserve square brackets after a value for indexing, and the parser
has no such form: `xs[0]` is a syntax error. Reading one document of a file,
`source(file("birds.json"))`, needs it, and so does jq's `.[0]`.

## Scope

- `xs[i]` reads the element at position `i` of a `Seq<T>`, counting from `0`,
  as `Option<T>`: a position past the end, or below `0`, is absence, as a
  missing key of a map is.
- `d[i]` reads a member of a `Data` tree that is a sequence, as `Option<Data>`.
  A tree that is not a sequence is an evaluation error, as reading a key of a
  tree that is not a map is.
- An optional receiver is lifted: `o[i]` over an `Option<Seq<T>>` is one
  `Option<T>`, not two.
- `i` is an `Int`. Any other index is a check error.
- A `Set`, a `Map` or a `Collection` has no positions, so indexing one is a
  check error naming `sort`. A stream that may not end is refused, naming
  `take(n)`.
- `[` indexes only when it follows the value with no space between them, so a
  list literal on the next line stays a statement of its own.

## Done when

- The core card states the form and its type, and stops calling it reserved.
- `examples/birds/queries/` has a valid case and an invalid one.
- Tests cover a sequence, a tree, absence past the end, an optional receiver,
  and each refusal.
