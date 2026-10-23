// status: invalid
// feature: A standard constructor passed as a value is refused where the element type picks no one form
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `lib/std/graph/mod.hql` declares a Graph from cards, from hits and from edges. An
// empty list is each of them, so `map(Graph)` over lists of nothing is
// ambiguous.
[[]] | map(Graph)
