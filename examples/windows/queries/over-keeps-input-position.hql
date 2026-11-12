// status: valid-now
// feature: `over` gives one value per row at the row's input position; `row_number` is the row's place in the order
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [3, 1, 2]

[30, 10, 20] | over(order = x => x, into = (x, w) => w | row_number)
