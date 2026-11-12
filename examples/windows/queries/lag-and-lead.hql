// status: valid-now
// feature: `lag` and `lead` are the rows before and after, `none` past either end
// implementation: implemented
// environment: pure
// expected-type: List<List<Int>>
// expected: [[0, 4], [1, 9], [4, 0]]

[1, 4, 9]
| over(order = x => x,
       into = (x, w) => [w | lag | unwrap_or(0), w | lead | unwrap_or(0)])
