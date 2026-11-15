// status: valid-now
// feature: import stats binds the module name, so its functions are called qualified, piped qualified, or as receivers
// implementation: implemented
// environment: pure
// expected-type: List<Option<Float>>
// expected-json: [2.5, 2.5, 2.5]

import stats

[stats.mean([1, 2, 3, 4]), [1, 2, 3, 4] | stats.median, [1, 2, 3, 4].mean()]
