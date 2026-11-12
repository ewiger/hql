// status: valid-now
// feature: a wall-clock reading in a DST gap moves forward by the gap
// implementation: implemented
// environment: pure
// expected-type: Instant
// expected: 2026-03-29T01:30:00Z

import time

// 02:30 does not happen in Berlin on 2026-03-29; it reads as 03:30 CEST.
DateTime("2026-03-29T02:30:00").in_zone(Zone("Europe/Berlin"))
