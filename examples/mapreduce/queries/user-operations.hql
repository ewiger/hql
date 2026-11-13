// status: valid-now
// feature: A program's own operations, bound at the operation types, are steps beside filter and map
// implementation: implemented
// environment: pure
// expected-type: Int
// expected-json: 2084

// A value of `Elementwise<I, O>` is a function from `I` to `O` that is also
// an elementwise operation: piped a collection of `I`, it runs over each
// element, as `map` would, and its body reads its one element and nothing
// else. `reading` is a named function bound at the type, `long` a lambda.
// `total` is a `Reduction<List<Int>, Int>`, applied to the list whole; that
// its combiner splits is the author's claim. `hql plan` prints each step
// with the shape its type states and the stage the planner gives it.
fn minutes(pages: Int) : Int = pages * 2
reading : Elementwise<Int, Int> = minutes
long : Elementwise<Int, Bool> = n => n > 300
total : Reduction<List<Int>, Int> = xs => xs | sum

pages = [412, 280, 310, 172, 320, 95]

pages
| filter(long)
| reading
| total
