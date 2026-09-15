// status: proposed
// feature: closure capture
// implementation: pending
// environment: knowledge-v1
// expected-type: List[Person]
// expected: alice, carol
// note: Candidate lexical capture of immutable minimum; no dynamic caller lookup.

minimum = 30
older = p => p.age > minimum
people | filter(older)
