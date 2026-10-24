// status: invalid
// feature: A Set has no positions, so it takes no slice
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// A slice is a part between two positions, and a Set has none: `sort` it
// into a List first.
seen = Set(["wren", "robin", "raven"])

seen[0:2]
