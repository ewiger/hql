// status: proposed
// implementation: pending
// proposal: doc/proposals/HQL-0024/README.md
// environment: pure
// feature: Ranges, generated streams, zip, enumerate, folds and predicates
// expected-type: List<String>
// expected-json: ["[1, 4, 9, 16, 25]","[0, 1, 1, 2, 3, 5, 8, 13, 21, 34]","[15, 30, 45]","a1,b2","0:x 1:y","24","[0, 1, 1, 2, 3, 5, 8, 13]","true","false","[10, 7, 4, 1]"]

// `range(a, b)` is the half-open sequence of Ints, `range(a, b, step)` the
// same with a stride. A block body yields a stream that `take` and
// `take_while` end. `zip` and `enumerate` pair values as `Pair`.
fn fib_from(a: Int, b: Int) : Stream<Int> {
    yield a
    for n in fib_from(b, a + b) {
        yield n
    }
}

squares = range(1, 6) | map(n => n * n)

[
    str(squares),
    str(fib_from(0, 1) | take(10)),
    str(range(1, 101) | filter(n => n % 15 == 0) | take(3)),
    zip(["a", "b"], [1, 2]).map(p => fmt("{}{}", p.first, p.second)).join(","),
    enumerate(["x", "y"]).map(p => fmt("{}:{}", p.first, p.second)).join(" "),
    str(range(1, 5).reduce(1, *)),
    str(fib_from(0, 1).take_while(n => n < 20)),
    str(range(0, 10).any(n => n > 8)),
    str(range(0, 10).all(n => n > 8)),
    str(range(10, 0, -3)),
]
