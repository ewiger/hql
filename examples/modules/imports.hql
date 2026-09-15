// status: design-question
// feature: module import
// implementation: pending
// environment: modules-v1
// expected-type: Graph
// expected: graph constructed from adult cards
// note: Module resolution/export contracts are candidates; fixture module interfaces are documented.

import people
import graph
people.adults
| graph.fromCards
