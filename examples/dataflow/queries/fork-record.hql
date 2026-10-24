// status: valid-now
// feature: `xs | fork { … }` gives every branch the same input, and its value is the record of the branch results, each typed by its own pipeline
// implementation: implemented
// environment: pure
// expected-type: {small: List<Int>, large: List<Int>, count: Int}
// expected: {small: [1, 2], large: [30, 40, 50], count: 5}

[1, 2, 3, 4, 5]
| fork {
    small = filter(x => x < 3)
    large = filter(x => x >= 3) | map(x => x * 10)
    count = size()
}
