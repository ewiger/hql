// status: invalid
// feature: A type name no declaration introduces is a name error in a signature
// implementation: implemented
// environment: pure
// error: NameError
// stage: typecheck

// `Strng` names nothing. The signature is rejected where the type is
// written, before the body or the call is looked at.
fn f(x: Strng) : Int = 1
f(1)
