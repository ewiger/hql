// status: valid-now
// feature: parse_date, parse_datetime and parse_instant without a pattern read what to_date, to_datetime and to_instant read
// implementation: implemented
// environment: pure
// expected-type: List<Bool>
// expected: [true, true, true, true, true]

import time

[
  "1982-05-09".parse_date() == "1982-05-09".to_date(),
  "09.05.1982".parse_date() == none,
  "2026-10-24T09:30:00.25".parse_datetime() == "2026-10-24T09:30:00.25".to_datetime(),
  "2026-10-24T07:30:00Z".parse_instant() == "2026-10-24T07:30:00Z".to_instant(),
  "2026-10-24T09:30:00+02:00".parse_instant() == "2026-10-24T09:30:00+02:00".to_instant()
]
