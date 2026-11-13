// status: invalid
// feature: `merge` is for two maps or two streams; two lists are joined with `concat`, which keeps their order
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

[1, 2] | merge([3])
