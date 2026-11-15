// status: invalid
// feature: import stats binds the module name only, so a bare mean is refused, naming both fixes
// implementation: implemented
// environment: pure
// phase: check
// kind: unknown-name
// severity: error

import stats

[1, 2, 3] | mean
