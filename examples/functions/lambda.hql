// status: proposed
// feature: lambda
// implementation: pending
// environment: knowledge-v1
// expected-type: List[String]
// expected: Alice, Carol

people
| filter(p => p.age > 30)
| map(p => p.name)
