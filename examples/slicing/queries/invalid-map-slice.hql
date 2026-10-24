// status: invalid
// feature: A Map has keys, not positions, so it takes no slice
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// A Map is read by key with `get`; a range of positions means nothing over
// it.
counts = Map(["wren", "robin"], [2, 1])

counts[:1]
