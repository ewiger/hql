// status: valid-now
// feature: One pipeline as an `=` body and as a block body, the two giving one value
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [12, 12]

// The two body forms declare the same function. In a block, a pipeline may
// continue over several lines, each opening with `|`.
fn doubled_sum(xs: List<Int>) : Int = xs | map(x => x * 2) | sum

fn doubled_sum_block(xs: List<Int>) : Int {
    xs
    | map(x => x * 2)
    | sum
}

[doubled_sum([1, 2, 3]), doubled_sum_block([1, 2, 3])]
