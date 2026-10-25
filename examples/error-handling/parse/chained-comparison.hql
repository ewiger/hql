// status: invalid
// feature: Comparisons do not chain: the parser rejects the second operator
// implementation: implemented
// environment: pure
// error: SyntaxError
// stage: parse

// `1 < 2 < 3` is not `(1 < 2) < 3` with a Bool on the left of `<`, which
// would be a type error; it is not a sentence of the language at all.
1 < 2 < 3
