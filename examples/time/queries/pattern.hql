// status: valid-now
// feature: fmt writes a strftime pattern, and parse_date reads it back
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected: ["Saturday, 24 October 2026", "true"]

import time

d = Date("2026-10-24")
[d.fmt("%A, %d %B %Y"), str(d.fmt("%d.%m.%Y").parse_date("%d.%m.%Y") == some(d))]
