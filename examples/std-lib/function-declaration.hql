// status: design-question
// feature: function declaration
// implementation: pending
// environment: pure
// expected-type: Int
// expected: 5
// note: fn names parameters and a return type in one place, which the arrow-typed binding form cannot do. The open part is arity: this declaration is not curried, so add(2) is an arity error rather than a partially applied function, and that differs from the arrow form it competes with.
// alternatives: types/function.hql

fn add(a : Int, b : Int) -> Int = a + b

add(2, 3)
