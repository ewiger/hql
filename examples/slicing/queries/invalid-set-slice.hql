// status: invalid
// feature: A Set has no positions, so it takes no slice
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// A slice is a part between two positions, and a Set has none: `sort` it
// into a List first.
seen = Set(["wren", "robin", "raven"])

seen[0:2]
