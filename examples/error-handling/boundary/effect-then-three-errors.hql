// status: invalid
// feature: Three check errors are listed under collect, and nothing runs
// implementation: implemented
// environment: pure
// phase: check
// kind: unknown-name
// severity: error

// A name error, a type error and an `Option` used as its value. `strict`
// stops at the first; `--report collect` lists all three. Neither prints
// `before`.
["before"] | target(stdout, Lines)

a = missing + 1
b = "a" + 1
c = [1, 2].first() + 1
[a, b, c]
