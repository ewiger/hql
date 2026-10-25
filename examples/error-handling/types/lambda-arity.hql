// status: invalid
// feature: A lambda takes as many parameters as the step gives it
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `map` gives one element. A lambda of two parameters is rejected.
[1] | map((a, b) => a)
