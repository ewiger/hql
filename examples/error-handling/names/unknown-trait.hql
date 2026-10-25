// status: invalid
// feature: An implementation of a trait not in scope is a name error at the impl
// implementation: implemented
// environment: pure
// phase: check
// kind: unknown-name
// severity: error

// `Nope` names no trait, so there is nothing to verify the implementation
// against, and the checker rejects the `impl` line itself.
impl Nope for Int {
    fn f(self) : Int = 1
}
1
