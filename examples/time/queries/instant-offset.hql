// status: valid-now
// feature: an Instant is a point on the UTC timeline, whatever offset its text was written with
// implementation: implemented
// environment: pure
// expected-type: Bool
// expected: true

Instant("2026-10-24T09:30:00+02:00") == Instant("2026-10-24T07:30:00Z")
