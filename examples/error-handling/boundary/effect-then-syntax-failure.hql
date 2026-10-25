// status: invalid
// feature: A syntax failure in the last line keeps the first line from running
// implementation: implemented
// environment: pure
// error: SyntaxError
// stage: parse

// `target(stdout, Lines)` writes `before` the moment it runs. It never
// runs: the parser rejects line 4, so there is no program to check or
// execute, and `hql run` prints the diagnostic alone.
["before"] | target(stdout, Lines)

x = (1 +
x
