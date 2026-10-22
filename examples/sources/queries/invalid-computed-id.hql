// status: invalid
// feature: a source id is a literal, so its type is known before evaluation
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

id = "counts"
source(id)
