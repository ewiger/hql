// status: proposed
// implementation: pending
// proposal: doc/proposals/HQL-0024/README.md
// environment: pure
// feature: Integer arithmetic, conversions and the math module
// expected-type: List<Int>
// expected: [3, 1, -1, 1024, 3, 12, 1024, 10, 6, 7, 9]

// Division truncates toward zero and `%` keeps the sign of the dividend.
// `int` and `float` convert between the two number types; nothing widens
// on its own. `math` is imported, and `abs` and `max` are receiver
// functions of the prelude.
import math

[
    7 / 2,
    7 % 3,
    -7 % 3,
    2 * 2 * 2 * 2 * 2 * 2 * 2 * 2 * 2 * 2,
    int(3.9),
    int(math.sqrt(144.0)),
    math.pow(2, 10),
    (-10).abs(),
    math.gcd(48, 18),
    7.max(3),
    (1 + 2) * 3,
]
