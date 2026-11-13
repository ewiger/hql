// status: valid-now
// feature: a Date is built from ISO 8601 text or from its parts
// implementation: implemented
// environment: pure
// expected-type: List<Date>
// expected: [2026-10-24, 2026-02-28, 2026-01-15]

import time

[Date("2026-10-24"), Date(year = 2026, month = 2, day = 28), "2026-01-15".to_date() | unwrap_or(Date("1970-01-01"))]
