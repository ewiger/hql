// status: proposed
// feature: multiline binding
// implementation: pending
// environment: knowledge-v1
// expected-type: List[Person]
// expected: alice, carol
// note: Indentation and expression termination remain grammar questions.

adults =
    people
    | filter(.age >= 18)
adults
