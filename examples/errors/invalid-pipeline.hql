// status: invalid
// feature: diagnostic
// implementation: pending
// environment: knowledge-v1
// error: PipelineTypeMismatch
// stage: typecheck
// note: A collection stage cannot consume Int; filter(42) is also an invalid predicate application.

42
| filter(.type == "person")
