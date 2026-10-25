# 0058 — A value read from a card loses where it came from

Status: todo
Related: [HQL-0045](../proposals/HQL-0045/README.md),
[`src/document.rs`](../../src/document.rs),
[`src/knowledge.rs`](../../src/knowledge.rs),
[`src/evaluation.rs`](../../src/evaluation.rs)

A loaded card keeps its path and nothing else about where its values are.
`Document::parse` reads the header into a `Data` tree with no positions, and
`c.metadata.pages` evaluates to a plain `Int`. A diagnostic's `source` is
`{ path, line?, key? }`, and the evaluator can fill only `path`, and only
while the card itself is bound in the function or lambda that failed. Once a
value has left the card, its origin is gone:

```hql
cards
| filter(c => c.metadata.category == "book")
| map(c => match c.metadata.pages { some(Int n) => n, _ => 1 })
| map(n => 1000 / (n - 112))
```

```text
execute failure[division-by-zero]: division by zero
  = element: 112, position 1 of 7 in `map`
  = bindings: n = 112
```

Run with `--vault examples/tutorial`, there is no `source` at all, though
`112` is `metadata.pages` on line 8 of
`examples/tutorial/books/a-room-of-ones-own.hmd`.

## Work

1. Every card carries system metadata from the loader: its path, and for
   each header key the line it is written on, kept beside the authored
   header rather than inside it.
2. A value read from a card's header keeps its provenance — the card's path,
   the key path and the line — through field access and the steps after it,
   so a diagnostic about it fills `source` with all three wherever it fails.
3. The text form shows it as `= source: path:line  key`, the JSON form as
   `{ "path", "line", "key" }`.
4. A case under `examples/error-handling/diagnostics/`, written first as
   `proposed`: the pipeline above, its diagnostic naming
   `books/a-room-of-ones-own.hmd`, line 8 and `metadata.pages`.
