// status: valid-now
// feature: a pure branch nothing reads is not run: the division by zero in `inverse` never happens
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [1, 2, 3]

parts = [0, 1, 2] | fork { shifted = map(x => x + 1), inverse = map(x => 10 / x) }
parts.shifted
