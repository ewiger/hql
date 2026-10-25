// status: invalid
// feature: Data that does not read as the declared type fails the run where it is read
// implementation: implemented
// environment: pure
// expected-type: Int
// phase: execute
// kind: contract-violation
// severity: error

// `u | User` reads a `Data` record as a `User`, fail-fast. The checker
// cannot see inside a `Data` value, so the shape is checked when the value
// is read, and `"forty"` is reported against `age`. `evaluation/handled-read.hql`
// reads the same data with `validate`, as a value.
type User { name : String, age : Int }

users = [{name: "ada", age: 36}, {name: "bob", age: "forty"}]

users | map(u => u | User) | map(u => u.age) | sum
