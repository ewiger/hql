// status: invalid
// feature: `retry` wraps the operation, never a finished `Result`: a list of results gives it nothing to invoke again
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

results : List<Result<Int, String>> = [err("busy")]
results | retry(attempts = 3)
