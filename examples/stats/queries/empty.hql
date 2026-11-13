// status: valid-now
// feature: a statistic undefined over its input is none
// implementation: implemented
// environment: pure
// expected-type: List<Option<Float>>
// expected: [none, none, none]

import stats

none_above = [1, 2, 3] | filter(x => x > 5)
[none_above | mean, [4] | stdev, zip([1, 2], [3, 3]) | correlation]
