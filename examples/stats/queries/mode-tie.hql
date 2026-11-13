// status: valid-now
// feature: among equally frequent values the mode is the least
// implementation: implemented
// environment: pure
// expected-type: Option<Int>
// expected: 1

import stats

[3, 1, 3, 1, 2] | mode
