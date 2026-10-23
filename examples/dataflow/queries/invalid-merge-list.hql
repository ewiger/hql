// status: invalid
// feature: `merge` is for two maps or two streams; two lists are joined with `concat`, which keeps their order
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

[1, 2] | merge([3])
