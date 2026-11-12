// status: valid-now
// feature: Durations add with + and compare by length
// implementation: implemented
// environment: pure
// expected-type: Duration
// expected: PT1H45M

Duration("PT1H") + Duration(minutes = 30) + Duration("PT15M")
