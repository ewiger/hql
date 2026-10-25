// status: invalid
// feature: A pipeline step no provider declares is a name error, with the nearest step
// implementation: implemented
// environment: pure
// phase: check
// kind: unknown-step
// severity: error

// `mapp` is not a step. The step name resolves before its lambda is typed.
[1, 2] | mapp(n => n)
