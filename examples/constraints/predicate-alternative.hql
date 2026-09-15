// status: design-question
// feature: predicate validation
// implementation: pending
// environment: knowledge-v1
// expected-type: Validation[Person]
// expected: Success(alice)
// note: Ordinary function plus Validation value is an alternative to constraint/check syntax.
// alternatives: constraints/satisfied.hql

adult : Person -> Bool = p => p.age >= 18
validate(alice, adult)
