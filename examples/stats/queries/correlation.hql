// status: valid-now
// feature: covariance and Pearson correlation of pairs
// implementation: implemented
// environment: pure
// expected-type: List<Option<Float>>
// expected: [3.2, 0.7917946548886297]

import stats

pairs = zip([1, 2, 3, 4, 5, 6], [2, 1, 4, 3, 7, 5])
[pairs | covariance, pairs | correlation]
