// status: valid-now
// feature: mean of Int values is a Float, the exact mean rounded once
// implementation: implemented
// environment: pure
// expected-type: Option<Float>
// expected: 5.0

import stats

[2, 4, 4, 4, 5, 5, 7, 9] | mean
