// status: valid-now
// feature: a record declared under a contract has its fields and its rules, and its own rules read both
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [5, 9]

type Span : DataContract = {
    low: Int
    high: Int

    require low <= high
}

// `Window` reads `low` and `high` as its own, and `width` is checked
// against them.
type Window : Span = {
    width: Int

    require width == high - low
}

xs : List<Data> = [{low: 0, high: 5, width: 5}, {low: 1, high: 10, width: 9}]
xs | Window | map(w => w.width) | List
