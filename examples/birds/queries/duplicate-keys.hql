// status: invalid
// feature: Map construction rejects repeated keys
// implementation: implemented
// environment: pure
// expected-type: Map<String, Int>
// phase: execute
// kind: invalid-argument
// severity: failure

// Repeated observations belong in a List, or in counts stored as values.
// A Map cannot associate the same species name with two separate values.
Map(["barn-owl", "barn-owl"], [2, 1])
