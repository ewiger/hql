// status: invalid
// feature: A bracket holds a position or a range, never nothing
// implementation: implemented
// environment: pure
// phase: parse
// kind: unexpected-token
// severity: failure

// `xs[]` names neither a position nor a range; `xs[:]` is the whole.
sightings = ["wren", "robin"]

sightings[]
