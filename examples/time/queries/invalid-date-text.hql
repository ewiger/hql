// status: invalid
// feature: a constructor refuses text that is not a date, naming it
// implementation: implemented
// environment: pure
// phase: execute
// kind: invalid-argument
// severity: failure
// expected-type: Date

Date("2026-02-30")
