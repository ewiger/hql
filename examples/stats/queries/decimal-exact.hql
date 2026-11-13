// status: valid-now
// feature: a statistic of Decimal values is an exact Decimal
// implementation: implemented
// environment: pure
// expected-type: List<Option<Decimal>>
// expected: [3.22, 2.0806, 2.60075]

import stats

prices = [1.10, 2.20, 3.30, 4.45, 5.05]
[prices | mean, prices | pvariance, prices | variance]
