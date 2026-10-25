// status: invalid
// feature: A given condition is a Bool
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `1` is not a condition. The diagnostic also says what to do with an
// `Option<Bool>`: `unwrap_or` names what absence means.
fn pick(flag: Bool) : Int = 1 given flag otherwise 2
pick(true) given 1 otherwise 2
