// status: invalid
// feature: only Duration takes + among the time types
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

Date("2026-10-24") + Date("2026-10-24")
