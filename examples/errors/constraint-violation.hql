// status: invalid
// feature: diagnostic
// implementation: pending
// environment: knowledge-v1
// error: UnsatisfiedConstraint
// stage: validate
// note: Bob is 16 in the Person fixture; declaration syntax is tentative.

constraint adult(p: Person) =
    p.age >= 18

check adult(bob)
