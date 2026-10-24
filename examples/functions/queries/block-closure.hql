// status: valid-now
// feature: A block's bindings are local, and a lambda on its last line captures them
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [21, 25, 100]

// `k` is visible from the line after its binding to the end of the block and
// nowhere else; the outer `k` is untouched. The lambda the block returns
// keeps the local `k`, and takes its parameter type from the declared
// result, `Int -> Int`, as it would after `=`.
k = 100

fn adder(n: Int) : Int -> Int {
    k = n * 10
    x => x + k
}

add = adder(2)
[add(1), add(5), k]
