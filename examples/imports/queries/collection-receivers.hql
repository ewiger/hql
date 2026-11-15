// status: proposed
// feature: Collection.* selects every stats function whose self is a Collection
// implementation: pending
// proposal: doc/proposals/HQL-0060/index.hmd
// environment: pure
// expected-type: List<Option<Float>>
// expected-json: [2.5, 2.5]

from stats import Collection.*

[median([1, 2, 3, 4]), [1, 2, 3, 4].mean()]
