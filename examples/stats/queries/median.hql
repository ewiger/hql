// status: valid-now
// feature: the median of an even count is the mean of the two middle values
// implementation: implemented
// environment: pure
// expected-type: Option<Float>
// expected: 2.5

import stats

[3, 1, 4, 2] | median
