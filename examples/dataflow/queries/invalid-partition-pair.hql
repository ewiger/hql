// status: invalid
// feature: `partition` is no longer a predicate's split into a pair: it keeps its input's type
// implementation: implemented
// environment: pure
// error: NameError
// stage: typecheck

([5, 3, 8, 1] | partition(by = x => x > 4)).first
