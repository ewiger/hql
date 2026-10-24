// status: invalid
// feature: A bracket holds a position or a range, never nothing
// implementation: implemented
// environment: pure
// error: SyntaxError
// stage: parse

// `xs[]` names neither a position nor a range; `xs[:]` is the whole.
sightings = ["wren", "robin"]

sightings[]
