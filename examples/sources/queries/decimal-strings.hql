// status: valid-now
// feature: a `Decimal` field reads a JSON string's digits, its default representation, and a JSON number's digits too
// implementation: implemented
// environment: knowledge-v1
// vault: sources/station
// expected-type: Decimal
// expected: 16.00

type Price {
    item  : String
    price : Decimal
}

prices : Seq<Price> = source("prices")
prices | map(p => p.price) | sum
