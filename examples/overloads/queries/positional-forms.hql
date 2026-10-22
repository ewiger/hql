// status: valid-now
// feature: Positional arguments of overloaded standard operations
// implementation: implemented
// environment: pure
// expected-type: List<Data>
// expected-json: [2, true, 8, 8.0]

// `count`, `contains`, `get` and `pow` take their second value by position.
// `2 | pow(3)` takes the Int form and `2.0 | pow(3.0)` the Float one.
[
    [1, 2, 1] | count(1),
    [1, 2] | contains(2),
    2 | pow(3),
    2.0 | pow(3.0)
]
