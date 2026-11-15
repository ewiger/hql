// status: valid-now
// feature: from stats import * except leaves out the named functions, so the prelude's variance stays unambiguous
// implementation: implemented
// environment: pure
// expected-type: List<Option<Float>>
// expected-json: [2.5, 1.25]

from stats import * except (variance, stdev)

[median([1, 2, 3, 4]), [1, 2, 3, 4] | variance]
