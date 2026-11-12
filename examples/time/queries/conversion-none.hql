// status: valid-now
// feature: to_date gives none for text that is not a date
// implementation: implemented
// environment: pure
// expected-type: List<Option<Date>>
// expected: [none, 2026-02-28]

import time

["2026-02-30".to_date(), "2026-02-28".to_date()]
