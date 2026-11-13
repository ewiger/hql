// status: valid-now
// feature: sample and population variance and standard deviation, as Python statistics gives them
// implementation: implemented
// environment: pure
// expected-type: List<Option<Float>>
// expected: [4.571428571428571, 2.138089935299395, 4.0, 2.0]

import stats

xs = [2, 4, 4, 4, 5, 5, 7, 9]
[xs | stats.variance, xs | stdev, xs | pvariance, xs | pstdev]
