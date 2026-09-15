// Fixture module; proposed syntax, not an executable corpus case.
// Implementation: pending. Explicit exports, no implicit card symbols.
type Person = Card & { name: String, age: Int }
export Person
export adults = cards | typed Person | filter(.age >= 18)
