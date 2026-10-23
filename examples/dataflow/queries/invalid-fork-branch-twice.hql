// status: invalid
// feature: a `fork` names each branch once
// implementation: implemented
// environment: pure
// error: SyntaxError
// stage: parse

[1, 2] | fork { a = size(), a = sum() }
