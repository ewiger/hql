// status: proposed
// feature: optional value declaration
// implementation: pending
// environment: pure
// expected-type: Option<Int>
// expected: Some(1)
// note: There is no null in the language: absence is an ordinary value of a sum type declared in the standard library, not a state every type secretly has. The variant bar collides with the pipe operator, which is the reason the alternative spelling is kept.
// alternatives: std-lib/sum-type-alternative.hql

type Option<T> =
    | Some(T)
    | None

first : Option<Int> = Some(1)
first
