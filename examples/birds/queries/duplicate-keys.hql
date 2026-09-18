// status: invalid
// feature: Map construction rejects repeated keys
// implementation: implemented
// environment: pure
// expected-type: Map<String, Int>
// error: RuntimeError
// stage: evaluate

// Repeated observations belong in a List, or in counts stored as values.
// A Map cannot associate the same species name with two separate values.
Map(["barn-owl", "barn-owl"], [2, 1])
