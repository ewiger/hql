// status: invalid
// feature: A type error in the second statement keeps the first from running
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// The first statement is well typed and effectful. The checker rejects the
// second, and the program executes nothing: `hql run` prints the diagnostic
// and no `before`. Under `collect` it is the same, with every error listed.
["before"] | target(stdout, Lines)

x = "a" + 1
x
