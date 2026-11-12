// status: valid-now
// feature: Dates sort and take max by time, not as text
// implementation: implemented
// environment: pure
// expected-type: List<Date>
// expected: [2025-12-31, 2026-01-01, 2026-10-24]

[Date("2026-10-24"), Date("2025-12-31"), Date("2026-01-01")] | sort
