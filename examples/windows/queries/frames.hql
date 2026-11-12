// status: valid-now
// feature: the frames of a window: `rows(1, 1)` clipped at the ends, `whole`, and the partition's `first`, `last` and `size`
// implementation: implemented
// environment: pure
// expected-type: List<List<Int>>
// expected: [[3, 1, 4, 4, 10], [6, 1, 4, 4, 10], [9, 1, 4, 4, 10], [7, 1, 4, 4, 10]]

[1, 2, 3, 4]
| over(order = x => x,
       into = (x, w) => [w | rows(1, 1) | sum, w | first, w | last, w | size, w | whole | sum])
