// status: valid-now
// feature: the 95th percentile by linear interpolation, exact over Decimal values
// implementation: implemented
// environment: pure
// expected-type: Option<Decimal>
// expected: 4.9300

import stats

[1.10, 2.20, 3.30, 4.45, 5.05] | quantile(0.95)
