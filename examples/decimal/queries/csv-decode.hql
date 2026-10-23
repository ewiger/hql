// status: valid-now
// feature: a CSV cell read as a Decimal is read as a Decimal literal
// implementation: implemented
// environment: pure
// io: sandbox
// expected-type: Decimal
// expected: 0.60

// Three prices of 0.20 sum to exactly 0.60.
type Line {
    item  : String
    price : Decimal
}

lines : Seq<Line> = source(file("../prices.csv"))
lines | map(l => l.price) | sum
