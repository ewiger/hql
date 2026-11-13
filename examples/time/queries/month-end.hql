// status: valid-now
// feature: add_months clamps to the last day of the month it lands in
// implementation: implemented
// environment: pure
// expected-type: List<Date>
// expected: [2026-02-28, 2024-02-29, 2025-02-28]

import time

[Date("2026-01-31").add_months(1), Date("2024-01-31").add_months(1), Date("2024-02-29").add_years(1)]
