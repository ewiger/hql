// status: design-question
// feature: constraint success
// implementation: pending
// environment: knowledge-v1
// expected-type: Unit
// expected: validation succeeds; no implicit output
// note: Constraint and language-level check syntax are future candidates; unrelated to implementing theorem proving or the current CLI check.

constraint adult(p: Person) =
    p.age >= 18

check adult(alice)
