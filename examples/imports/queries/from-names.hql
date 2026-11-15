// status: proposed
// feature: from stats import binds the listed functions unqualified, as a call and as a step
// implementation: pending
// proposal: doc/proposals/HQL-0060/index.hmd
// environment: pure
// expected-type: List<Option<Float>>
// expected-json: [2.5, 2.5]

from stats import mean, median

[mean([1, 2, 3, 4]), [1, 2, 3, 4] | median]
