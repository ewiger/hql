// status: valid-now
// feature: A filter whose predicate tests the element's type narrows the result
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected-json: [1, 2]

// `lib/std/collections/mod.hql` declares `filter(by: T -> Is<U>) : F<U>` before the
// general `filter(by: T -> Bool) : S`: a predicate written `x => x | is(U)`
// takes the first form, and the list of trees becomes a List<Int>.
[1, "two", 2] | filter(x => x | is(Int))
