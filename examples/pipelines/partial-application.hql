// status: proposed
// feature: pipeline argument placement
// implementation: pending
// environment: knowledge-v1
// expected-type: List[String]
// expected: Alice, Carol
// note: filter(predicate) and map(function) return functions on collections; no implicit printing.

people
| filter(p => p.age >= 18)
| map(p => p.name)
