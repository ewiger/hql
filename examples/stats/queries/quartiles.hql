// status: valid-now
// feature: quantiles(4) gives the three quartiles, as Python quantiles with method inclusive
// implementation: implemented
// environment: pure
// expected-type: List<Float>
// expected: [4.0, 4.5, 5.5]

import stats

[2, 4, 4, 4, 5, 5, 7, 9] | quantiles(4)
