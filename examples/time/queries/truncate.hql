// status: valid-now
// feature: truncate gives the start of the week, month, quarter or year
// implementation: implemented
// environment: pure
// expected-type: List<Date>
// expected: [2026-11-09, 2026-11-01, 2026-10-01, 2026-01-01]

import time

d = Date("2026-11-12")
[d.truncate(Week), d.truncate(Month), d.truncate(Quarter), d.truncate(Year)]
